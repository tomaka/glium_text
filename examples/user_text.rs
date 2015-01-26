extern crate glutin;
extern crate glium;
extern crate glium_text;
extern crate nalgebra;

use std::sync::Arc;
use glium::Surface;

fn main() {
    use glium::DisplayBuild;
    use std::io::File;

    let display = glutin::WindowBuilder::new().with_dimensions(1024, 768).build_glium().unwrap();
    let system = glium_text::TextSystem::new(&display);

    let font = Arc::new(match std::os::args().into_iter().nth(1) {
        Some(file) => glium_text::FontTexture::new(&display, File::open(&Path::new(file)), 70),
        None => {
            match File::open(&Path::new("C:\\Windows\\Fonts\\Arial.ttf")) {
                Ok(f) => glium_text::FontTexture::new(&display, f, 70),
                Err(_) => glium_text::FontTexture::new(&display, std::io::BufReader::new(include_bytes!("font.ttf")), 70),
            }
        }
    }.unwrap());

    let mut buffer = String::new();

    println!("Type with your keyboard");

    'main: loop {
        use std::io::timer;
        use std::time::Duration;

        let text = glium_text::TextDisplay::new(&system, font.clone(), buffer.as_slice());

        let (w, h) = display.get_framebuffer_dimensions();

        let matrix = nalgebra::Mat4::new(
            1.0, 0.0, 0.0, 0.0,
            0.0, 1.0 * (w as f32) / (h as f32), 0.0, 0.0,
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
                glutin::Event::ReceivedCharacter('\r') => buffer.clear(),
                glutin::Event::ReceivedCharacter(c) if c as u32 == 8 => { buffer.pop(); },
                glutin::Event::ReceivedCharacter(chr) => buffer.push(chr),
                glutin::Event::Closed => break 'main,
                _ => ()
            }
        }
    }
}
