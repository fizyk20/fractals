use std::cmp::max;

use iced::{executor, image::Handle, Application, Command, Element, Image, Length, Subscription};
use iced_native::{mouse, subscription::events_with, window, Event};
use image::{Bgra, ImageBuffer};
use num_complex::Complex;
use rayon::{iter::ParallelBridge, prelude::ParallelIterator};

const NUM_COLORS: u32 = 2048;

fn test_number(c: Complex<f64>, n: u32) -> Option<f32> {
    let mut z = Complex::new(0.0, 0.0);
    for i in 0..n {
        z = z * z + c;
        if z.norm() >= 16.0 {
            return Some(i as f32 + 1.0 - z.norm().log2().ln() as f32);
        }
    }
    None
}

fn color_palette(val: Option<f32>, n: u32) -> Bgra<u8> {
    match val {
        None => Bgra([0, 0, 0, 255]),
        Some(val) => {
            let fval = val / (n as f32);
            let fval = fval.sqrt();
            let pi = 3.14159265f32;
            let r = ((pi / 2.0 * fval).sin().powi(2) * 255.0) as u8;
            let g = ((3.0 * pi / 2.0 * fval).sin().powi(2) * 255.0) as u8;
            let b = ((7.0 * pi / 2.0 * fval).sin().powi(2) * 255.0) as u8;
            Bgra([b, g, r, 255])
        }
    }
}

#[derive(Clone, Copy)]
struct ViewState {
    center: Complex<f64>,
    scale: f64,
    width: u32,
    height: u32,
}

impl ViewState {
    fn xy_to_point(&self, x: u32, y: u32) -> Complex<f64> {
        let greater_dim = max(self.width, self.height) as f64;
        let width_ratio = (self.width as f64) / (greater_dim as f64);
        let height_ratio = (self.height as f64) / (greater_dim as f64);
        let x = ((x as f64 / greater_dim) - 0.5 * width_ratio) * self.scale;
        let y = (0.5 * height_ratio - (y as f64 / greater_dim)) * self.scale;

        self.center + Complex::new(x, y)
    }

    fn generate(&self) -> ImageBuffer<Bgra<u8>, Vec<u8>> {
        let mut image = ImageBuffer::new(self.width, self.height);

        image
            .enumerate_pixels_mut()
            .par_bridge()
            .for_each(|(x, y, pixel)| {
                let c = self.xy_to_point(x, y);
                let value = test_number(c, NUM_COLORS);
                *pixel = color_palette(value, NUM_COLORS);
            });

        image
    }
}

struct AppState {
    view_state: ViewState,
    image: ImageBuffer<Bgra<u8>, Vec<u8>>,
    cursor: (f32, f32),
    panning: Option<(f32, f32)>,
}

#[derive(Debug)]
enum Message {
    WindowResize { width: u32, height: u32 },
    MousePress,
    MouseRelease,
    MouseMove { x: f32, y: f32 },
    MouseScroll { delta: f32 },
}

impl Application for AppState {
    type Executor = executor::Default;
    type Flags = ();
    type Message = Message;

    fn new(_flags: ()) -> (Self, Command<Message>) {
        let view_state = ViewState {
            center: Complex::new(-0.5, 0.0),
            scale: 4.0,
            width: 10,
            height: 10,
        };
        (
            AppState {
                view_state,
                image: view_state.generate(),
                cursor: (0.0, 0.0),
                panning: None,
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        "Mandelbrot fractal viewer".to_string()
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::WindowResize { width, height } => {
                self.view_state.width = width;
                self.view_state.height = height;
                self.regenerate();
            }
            Message::MousePress => {
                self.panning = Some(self.cursor);
            }
            Message::MouseRelease => {
                if let Some((old_x, old_y)) = self.panning.take() {
                    let (x, y) = self.cursor;
                    let max_dim = max(self.view_state.width, self.view_state.height) as f64;
                    let dx = (x - old_x) as f64;
                    let dy = (y - old_y) as f64;
                    let pan = Complex::new(-dx, dy) / max_dim * self.view_state.scale;
                    self.view_state.center += pan;
                    self.regenerate();
                }
            }
            Message::MouseMove { x, y } => {
                self.cursor = (x, y);
            }
            Message::MouseScroll { delta } => {
                let factor = (delta as f64).exp();
                let point_under_cursor = self
                    .view_state
                    .xy_to_point(self.cursor.0 as u32, self.cursor.1 as u32);
                let center_diff = self.view_state.center - point_under_cursor;
                self.view_state.scale /= factor;
                self.view_state.center = point_under_cursor + center_diff / factor;
                self.regenerate();
            }
        }
        Command::none()
    }

    fn view(&mut self) -> Element<Message> {
        let handle = Handle::from_pixels(
            self.view_state.width,
            self.view_state.height,
            self.image.as_raw().clone(),
        );
        Image::new(handle)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        events_with(|event, _| match event {
            Event::Window(window::Event::Resized { width, height }) => {
                Some(Message::WindowResize { width, height })
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                Some(Message::MousePress)
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                Some(Message::MouseRelease)
            }
            Event::Mouse(mouse::Event::CursorMoved { x, y }) => Some(Message::MouseMove { x, y }),
            Event::Mouse(mouse::Event::WheelScrolled { delta }) => match delta {
                mouse::ScrollDelta::Lines { y, .. } | mouse::ScrollDelta::Pixels { y, .. } => {
                    Some(Message::MouseScroll { delta: y })
                }
            },
            _ => None,
        })
    }
}

impl AppState {
    fn regenerate(&mut self) {
        self.image = self.view_state.generate();
    }
}

fn main() {
    AppState::run(Default::default()).expect("should run successfully");
}
