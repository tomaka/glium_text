#![feature(phase)]

extern crate glutin;
extern crate glium;
extern crate glium_text;
extern crate nalgebra;

use std::sync::Arc;
use glium::Surface;

fn main() {
    use glium::DisplayBuild;

    let display = glutin::WindowBuilder::new().with_dimensions(1024, 768).build_glium().unwrap();
    let system = glium_text::TextSystem::new(&display);

    let font = Arc::new(glium_text::FontTexture::new(&display, std::io::BufReader::new(include_bytes!("font.ttf")), 70).unwrap());

    let text = glium_text::TextDisplay::new(&system, font, "Hello world!");

    'main: loop {
        use std::io::timer;
        use std::time::Duration;

        let (w, h) = (1024.0f32, 768.0f32);

        let matrix = nalgebra::Mat4::new(
            0.5, 0.0, 0.0, 0.0,
            0.0, 0.5 * h / w, 0.0, 0.0,
            0.0, 0.0, 0.5, 0.0,
            0.0, 0.0, 0.0, 1.0f32,
        );

        let mut target = display.draw();
        target.clear_color(0.0, 0.0, 0.0, 1.0);
        glium_text::draw(&text, &system, &mut target, matrix, (1.0, 1.0, 0.0, 1.0));
        target.finish();

        timer::sleep(Duration::milliseconds(17));

        for event in display.poll_events().into_iter() {
            match event {
                glutin::Event::Closed => break 'main,
                _ => ()
            }
        }
    }
}
