/*!

This crate allows you to easily write text.

Usage:

```no_run
# extern crate glium;
# extern crate glium_text;
# fn main() {
# let display: glium::Display = unsafe { std::mem::uninitialized() };
// The `TextSystem` contains the shaders and elements used for text display.
let system = glium_text::TextSystem::new(&display);

// Creating a `FontTexture`, which a regular `Texture` which contains the font.
// Note that loading the systems fonts is not covered by this library.
let font = glium_text::FontTexture::new(&display, std::io::File::open(&Path::new("my_font.ttf")), 24).unwrap();

// Creating a `TextDisplay` which contains the elements required to draw a specific sentence.
let text = glium_text::TextDisplay::new(&system, &font, "Hello world!");

// Finally, drawing the text is done with a `DrawCommand`.
// This draw command contains the matrix and color to use for the text.
display.draw().draw(glium_text::DrawCommand(&text, &system,
    [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ], [1.0, 1.0, 0.0, 1.0]));
# }
```

*/

#![feature(core)]
#![deny(missing_docs)]

extern crate libc;
extern crate "freetype-sys" as freetype;
#[macro_use]
extern crate glium;
extern crate nalgebra;

use nalgebra::Mat4;
use std::sync::Arc;

/// Texture which contains the characters of the font.
pub struct FontTexture {
    texture: glium::texture::Texture2d,
    character_infos: Vec<(char, CharacterInfos)>,
}

/// Object that contains the elements shared by all `TextDisplay` objects.
///
/// Required to create a `TextDisplay`.
pub struct TextSystem {
    display: glium::Display,
    program: glium::Program,
}

/// Object that will allow you to draw a text.
pub struct TextDisplay {
    display: glium::Display,
    texture: Arc<FontTexture>,
    vertex_buffer: Option<glium::VertexBuffer<VertexFormat>>,
    index_buffer: Option<glium::IndexBuffer>,
    total_text_width: f32,
    is_empty: bool,
}

// structure containing informations about a character of a font
#[derive(Copy, Clone, Debug)]
struct CharacterInfos {
    // coordinates of the character top-left hand corner on the font's texture
    coords: (f32, f32),

    // width and height of character in texture units
    size: (f32, f32),

    // number of texture units between the bottom of the character and the base line of text
    height_over_line: f32,
    // number of texture units at the left of the character
    left_padding: f32,
    // number of texture units at the right of the character
    right_padding: f32,
}

#[derive(Copy)]
struct VertexFormat {
    position: [f32; 2],
    tex_coords: [f32; 2],
}

implement_vertex!(VertexFormat, position, tex_coords);

impl FontTexture {
    /// Creates a new texture representing a font stored in a `FontTexture`.
    pub fn new<R: Reader>(display: &glium::Display, mut font: R, font_size: u32) -> Result<FontTexture, ()> {
        // building the freetype library
        // FIXME: call FT_Done_Library
        let library = unsafe {
            // taken from https://github.com/PistonDevelopers/freetype-rs/blob/master/src/library.rs
            extern "C" fn alloc_library(_memory: freetype::FT_Memory, size: libc::c_long) -> *mut libc::c_void {
                unsafe {
                    libc::malloc(size as libc::size_t)
                }
            }
            extern "C" fn free_library(_memory: freetype::FT_Memory, block: *mut libc::c_void) {
                unsafe {
                    libc::free(block)
                }
            }
            extern "C" fn realloc_library(_memory: freetype::FT_Memory,
                                          _cur_size: libc::c_long,
                                          new_size: libc::c_long,
                                          block: *mut libc::c_void) -> *mut libc::c_void {
                unsafe {
                    libc::realloc(block, new_size as libc::size_t)
                }
            }
            static mut MEMORY: freetype::FT_MemoryRec = freetype::FT_MemoryRec {
                user: 0 as *mut libc::c_void,
                alloc: alloc_library,
                free: free_library,
                realloc: realloc_library,
            };

            let mut raw = ::std::ptr::null_mut();
            if freetype::FT_New_Library(&MEMORY, &mut raw) != freetype::FT_Err_Ok {
                return Err(());
            }
            freetype::FT_Add_Default_Modules(raw);

            raw
        };

        // building the freetype face object
        let font = try!(font.read_to_end().map_err(|_| {}));

        let face: freetype::FT_Face = unsafe {
            let mut face = ::std::ptr::null_mut();
            let err = freetype::FT_New_Memory_Face(library, font.as_ptr(), font.len() as freetype::FT_Long, 0, &mut face);
            if err == freetype::FT_Err_Ok {
                face
            } else {
                return Err(());
            }
        };

        // computing the list of characters in the font
        let characters_list = unsafe {
            // TODO: unresolved symbol
            /*if freetype::FT_Select_CharMap(face, freetype::FT_ENCODING_UNICODE) != 0 {
                return Err(());
            }*/

            let mut result = Vec::new();

            let mut g: freetype::FT_UInt = std::mem::uninitialized();
            let mut c = freetype::FT_Get_First_Char(face, &mut g);

            while g != 0 {
                result.push(std::mem::transmute(c as u32));     // TODO: better solution?
                c = freetype::FT_Get_Next_Char(face, c, &mut g);
            }

            result
        };

        // building the infos
        let (texture_data, (texture_width, _texture_height), chr_infos) = unsafe {
            build_font_image(face, characters_list, font_size)
        };

        // we load the texture in the display
        let texture_data = texture_data.as_slice().chunks(texture_width as usize).map(|s| s.to_vec()).collect::<Vec<_>>();
        let texture = glium::texture::Texture2d::new(display, texture_data);

        Ok(FontTexture {
            texture: texture,
            character_infos: chr_infos,
        })
    }
}

impl<'a> glium::uniforms::IntoUniformValue<'a> for &'a FontTexture {
    fn into_uniform_value(self) -> glium::uniforms::UniformValue<'a> {
        (&self.texture).into_uniform_value()
    }
}

impl TextSystem {
    /// Builds a new text system that must be used to build `TextDisplay` objects.
    pub fn new(display: &glium::Display) -> TextSystem {
        TextSystem {
            display: display.clone(),
            program:
                glium::Program::from_source(display, r"
                    #version 110

                    attribute vec2 position;
                    attribute vec2 tex_coords;
                    varying vec2 v_tex_coords;
                    uniform mat4 matrix;

                    void main() {       // TODO: understand why '* 4.0' is needed
                        gl_Position = matrix * vec4(position.x * 4.0, position.y, 0.0, 1.0);
                        v_tex_coords = tex_coords;
                    }
                ", r"
                    #version 110

                    varying vec2 v_tex_coords;
                    uniform vec4 color;
                    uniform sampler2D texture;

                    void main() {
                        gl_FragColor = vec4(color.rgb, color.a * texture2D(texture, v_tex_coords));
                        if (gl_FragColor.a <= 0.01) {
                            discard;
                        }
                    }
                ", None).unwrap()
        }
    }
}

impl TextDisplay {
    /// Builds a new text display that allows you to draw text.
    pub fn new(system: &TextSystem, texture: Arc<FontTexture>, text: &str) -> TextDisplay {
        let mut text_display = TextDisplay {
            display: system.display.clone(),
            texture: texture,
            vertex_buffer: None,
            index_buffer: None,
            total_text_width: 0.0,
            is_empty: true,
        };

        text_display.set_text(text);

        text_display
    }

    /// Returns the width in GL units of the text.
    pub fn get_width(&self) -> f32 {
        self.total_text_width
    }

    /// Modifies the text on this display.
    pub fn set_text(&mut self, text: &str) {
        self.is_empty = true;
        self.total_text_width = 0.0;
        self.vertex_buffer = None;
        self.index_buffer = None;

        // returning if no text
        if text.len() == 0 {
            return;
        }

        // these arrays will contain the vertex buffer and index buffer data
        let mut vertex_buffer_data = Vec::with_capacity(text.len() * 4 * 4);
        let mut index_buffer_data = Vec::with_capacity(text.len() * 6);

        // iterating over the characters of the string
        for character in text.nfc_chars() {
            let infos = match self.texture.character_infos
                .iter().find(|&&(chr, _)| chr == character)
            {
                Some(infos) => infos,
                None => continue        // character not found in the font, ignoring it
            };
            let infos = infos.1;

            self.is_empty = false;

            // adding the quad in the index buffer
            {
                let first_vertex_offset = vertex_buffer_data.len() as u16;
                index_buffer_data.push(first_vertex_offset);
                index_buffer_data.push(first_vertex_offset + 1);
                index_buffer_data.push(first_vertex_offset + 2);
                index_buffer_data.push(first_vertex_offset + 2);
                index_buffer_data.push(first_vertex_offset + 1);
                index_buffer_data.push(first_vertex_offset + 3);
            }

            //
            self.total_text_width += infos.left_padding;

            // calculating coords
            let left_coord = self.total_text_width;
            let right_coord = left_coord + infos.size.0;
            let top_coord = infos.height_over_line;
            let bottom_coord = infos.height_over_line - infos.size.1;

            // top-left vertex
            vertex_buffer_data.push(VertexFormat {
                position: [left_coord, top_coord],
                tex_coords: [infos.coords.0, infos.coords.1],
            });

            // top-right vertex
            vertex_buffer_data.push(VertexFormat {
                position: [right_coord, top_coord],
                tex_coords: [infos.coords.0 + infos.size.0, infos.coords.1],
            });

            // bottom-left vertex
            vertex_buffer_data.push(VertexFormat {
                position: [left_coord, bottom_coord],
                tex_coords: [infos.coords.0, infos.coords.1 + infos.size.1],
            });

            // bottom-right vertex
            vertex_buffer_data.push(VertexFormat {
                position: [right_coord, bottom_coord],
                tex_coords: [infos.coords.0 + infos.size.0, infos.coords.1 + infos.size.1],
            });

            // going to next char
            self.total_text_width = right_coord + infos.right_padding;
        }

        if !vertex_buffer_data.len() != 0 {
            // building the vertex buffer
            self.vertex_buffer = Some(glium::VertexBuffer::new(&self.display, vertex_buffer_data));

            // building the index buffer
            self.index_buffer = Some(glium::IndexBuffer::new(&self.display, glium::index_buffer::TrianglesList(index_buffer_data)));
        }
    }
}

///
/// ## About the matrix
///
/// One unit in height corresponds to a line of text, but the text can go above or under.
/// The bottom of the line is 0, the top is 1.
/// You need to adapt your matrix by taking these into consideration.
pub fn draw<S>(text: &TextDisplay, system: &TextSystem, target: &mut S, matrix: Mat4<f32>, color: (f32, f32, f32, f32)) where S: glium::Surface {
    let &TextDisplay { ref vertex_buffer, ref index_buffer, ref texture, is_empty, .. } = text;
    let color = [color.0, color.1, color.2, color.3];

    // returning if nothing to draw
    if is_empty || vertex_buffer.is_none() || index_buffer.is_none() {
        return;
    }

    let vertex_buffer = vertex_buffer.as_ref().unwrap();
    let index_buffer = index_buffer.as_ref().unwrap();

    let uniforms = uniform! {
        matrix: matrix,
        color: color,
        texture: glium::uniforms::Sampler(&texture.texture, glium::uniforms::SamplerBehavior {
            magnify_filter: glium::uniforms::MagnifySamplerFilter::Linear,
            minify_filter: glium::uniforms::MinifySamplerFilter::Linear,
            .. std::default::Default::default()
        })
    };

    target.draw(vertex_buffer, index_buffer, &system.program, &uniforms, &std::default::Default::default()).unwrap();
}

unsafe fn build_font_image(face: freetype::FT_Face, characters_list: Vec<char>, font_size: u32) -> (Vec<f32>, (u32, u32), Vec<(char, CharacterInfos)>) {
    use std::iter;
    use std::num::Float;

    // setting the right pixel size
    if freetype::FT_Set_Pixel_Sizes(face, font_size, font_size) != 0 {
        panic!();
    }

    // this variable will store the texture data
    // we set an arbitrary capacity that we think will match what we will need
    let mut texture_data: Vec<f32> = Vec::with_capacity(characters_list.len() * font_size as usize * font_size as usize);

    // the width is chosen more or less arbitrarily, because we can store everything as long as the texture is at least as wide as the widest character
    // we just try to estimate a width so that width ~= height
    let texture_width = get_nearest_po2(std::cmp::max(font_size * 2 as u32, ((((characters_list.len() as u32) * font_size * font_size) as f32).sqrt()) as u32));

    // we store the position of the "cursor" in the destination texture
    // this cursor points to the top-left pixel of the next character to write on the texture
    let mut cursor_offset = (0u32, 0u32);

    // number of rows to skip at next carriage return
    let mut rows_to_skip = 0u32;

    // now looping through the list of characters, filling the texture and returning the informations
    let mut characters_infos: Vec<(char, CharacterInfos)> = characters_list.into_iter().filter_map(|character| {
        // loading wanted glyph in the font face
        if freetype::FT_Load_Glyph(face, freetype::FT_Get_Char_Index(face, character as freetype::FT_ULong), freetype::FT_LOAD_RENDER) != 0 {
            return None;
        }
        let bitmap = &(*(*face).glyph).bitmap;

        // carriage return our cursor if we don't have enough room to write the next caracter
        if cursor_offset.0 + (bitmap.width as u32) >= texture_width {
            assert!(bitmap.width as u32 <= texture_width);       // if this fails, we should increase texture_width
            cursor_offset.0 = 0;
            cursor_offset.1 += rows_to_skip;
            rows_to_skip = 0;
        }

        // if the texture data buffer has not enough lines, adding some
        if rows_to_skip < bitmap.rows as u32 {
            let diff = (bitmap.rows as u32) - rows_to_skip;
            rows_to_skip = bitmap.rows as u32;
            texture_data.extend(iter::repeat(0.0).take((diff * texture_width) as usize));
        }

        // copying the data to the texture
        let offset_x_before_copy = cursor_offset.0;
        if bitmap.rows >= 1 {
            let destination = &mut texture_data[(cursor_offset.0 + cursor_offset.1 * texture_width) as usize ..];
            let source = std::mem::transmute(bitmap.buffer);
            let source = std::slice::from_raw_parts(source, destination.len());

            for y in range(0, bitmap.rows as u32) {
                let source = &source[(y * bitmap.width as u32) as usize ..];
                let destination = &mut destination[(y * texture_width) as usize ..];

                for x in range(0, bitmap.width) {
                    // the values in source are bytes between 0 and 255, but we want floats between 0 and 1
                    let val: u8 = *source.get(x as usize).unwrap();
                    let max: u8 = std::num::Int::max_value();
                    let val = (val as f32) / (max as f32);
                    let dest = destination.get_mut(x as usize).unwrap();
                    *dest = val;
                }
            }

            cursor_offset.0 += bitmap.width as u32;
            debug_assert!(cursor_offset.0 <= texture_width);
        }

        // filling infos about that character
        // all informations are in pixels for the moment
        // when the texture dimensions will be determined, we will divide those by it
        let left_padding = (*(*face).glyph).bitmap_left;

        Some((character, CharacterInfos {
            left_padding: left_padding as f32,
            right_padding: (((*(*face).glyph).advance.x >> 6) as i32 - bitmap.width - left_padding) as f32,
            height_over_line: (*(*face).glyph).bitmap_top as f32,
            size: (bitmap.width as f32, bitmap.rows as f32),
            coords: (offset_x_before_copy as f32, cursor_offset.1 as f32),
        }))
    }).collect();

    // adding blank lines at the end until the height of the texture is a power of two
    {
        let current_height = texture_data.len() as u32 / texture_width;
        let requested_height = get_nearest_po2(current_height);
        texture_data.extend(iter::repeat(0.0).take((texture_width * (requested_height - current_height)) as usize));
    }

    // now our texture is finished
    // we know its final dimensions, so we can divide all the pixels values into (0,1) range
    assert!((texture_data.len() as u32 % texture_width) == 0);
    let texture_height = (texture_data.len() as u32 / texture_width) as f32;
    let float_texture_width = texture_width as f32;
    for chr in characters_infos.iter_mut() {
        chr.1.left_padding /= float_texture_width;
        chr.1.right_padding /= float_texture_width;
        chr.1.height_over_line /= texture_height;
        chr.1.size.0 /= float_texture_width;
        chr.1.size.1 /= texture_height;
        chr.1.coords.0 /= float_texture_width;
        chr.1.coords.1 /= texture_height;
    }

    // returning
    (texture_data, (texture_width, texture_height as u32), characters_infos)
}

/// Function that will calculate the nearest power of two.
fn get_nearest_po2(mut x: u32) -> u32 {
    assert!(x > 0);
    x -= 1;
    x = x | (x >> 1);
    x = x | (x >> 2);
    x = x | (x >> 4);
    x = x | (x >> 8);
    x = x | (x >> 16);
    x + 1
}
