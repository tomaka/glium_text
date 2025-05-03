extern crate glium;
extern crate glium_text;
extern crate cgmath;

use glium::{winit, Surface};

fn main() {
    let event_loop = winit::event_loop::EventLoop::builder().build().unwrap();

    let (window, display) = glium::backend::glutin::SimpleWindowBuilder::new()
        .with_inner_size(
            1024,
            768,
        )
        .build(&event_loop);

    let system = glium_text::TextSystem::new(&display);
    let font = glium_text::FontTexture::new(&display, &include_bytes!("font.ttf")[..], 70).unwrap();

    let mut buffer = String::new();

    println!("Type with your keyboard");

    #[allow(deprecated)]
    event_loop.run(move |event, window_target| {
        match event {
            glium::winit::event::Event::WindowEvent { event, .. } => match event {
                glium::winit::event::WindowEvent::CloseRequested => {
                    window_target.exit();
                },
                glium::winit::event::WindowEvent::KeyboardInput {
                    event: glium::winit::event::KeyEvent { logical_key: key, state: glium::winit::event::ElementState::Pressed, .. },
                    ..
                } => match key.as_ref() {
                    glium::winit::keyboard::Key::Named(glium::winit::keyboard::NamedKey::Enter) => {
                        buffer.clear();
                        window.request_redraw();
                    },
                    glium::winit::keyboard::Key::Character(chr) => {
                        buffer.push_str(chr);
                        window.request_redraw();
                    },
                    _ => (),
                },
                glium::winit::event::WindowEvent::RedrawRequested => {
                    let text = glium_text::TextDisplay::new(&system, &font, &buffer);

                    let (w, h) = display.get_framebuffer_dimensions();

                    let matrix:[[f32; 4]; 4] = cgmath::Matrix4::new(
                        0.1, 0.0, 0.0, 0.0,
                        0.0, 0.1 * (w as f32) / (h as f32), 0.0, 0.0,
                        0.0, 0.0, 1.0, 0.0,
                        -0.9, 0.0, 0.0, 1.0f32,
                    ).into();

                    let mut target = display.draw();
                    target.clear_color(0.0, 0.0, 0.0, 1.0);
                    glium_text::draw(&text, &system, &mut target, matrix, (1.0, 1.0, 0.0, 1.0));
                    target.finish().unwrap();
                }
                _ => (),
            },
            _ => (),
        }
    })
    .unwrap();
}
