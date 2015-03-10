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

    let font = Arc::new(glium_text::FontTexture::new(&display, std::old_io::BufReader::new(include_bytes!("font.ttf")), 70).unwrap());

    let text = glium_text::TextDisplay::new(&system, font, "Hello world!");
    println!("Text width: {:?}", text.get_width());

    'main: loop {
        use std::old_io::timer;
        use std::time::Duration;

        let (w, h) = display.get_framebuffer_dimensions();

        let matrix = nalgebra::Mat4::new(
            1.0, 0.0, 0.0, -1.0,
            0.0, 1.0 * (w as f32) / (h as f32), 0.0, -1.0,
            0.0, 0.0, 1.0, 0.0,
            0.0, 0.0, 0.0, 1.0f32,
        );

        let mut target = display.draw();
        target.clear_color(0.0, 0.0, 0.0, 1.0);
        glium_text::draw(&text, &system, &mut target, matrix, (1.0, 1.0, 0.0, 1.0));
        target.finish();

        timer::sleep(Duration::milliseconds(17));

        for event in display.poll_events() {
            match event {
                glutin::Event::Closed => break 'main,
                _ => ()
            }
        }
    }
}
