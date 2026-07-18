use aviutl2::AnyResult;
use aviutl2::module::{AsScriptModuleUserData, ScriptModuleFunctions, ScriptModuleUserData};

mod render;

// define VectorCanvasBackend
#[aviutl2::plugin(ScriptModule)]
struct VectorCanvasBackend;

// implement AviUtl2 Script Module Trait
impl aviutl2::module::ScriptModule for VectorCanvasBackend {
    fn new(_info: aviutl2::AviUtl2Info) -> AnyResult<Self> {
        Ok(VectorCanvasBackend)
    }
    fn plugin_info(&self) -> aviutl2::module::ScriptModuleTable {
        aviutl2::module::ScriptModuleTable {
            information: format!(
                "VectorCanvas Backend Module / v{version}",
                version = env!("CARGO_PKG_VERSION")
            ),
            functions: Self::functions(),
        }
    }
}

// pixel buffer for rendering
struct PixelBuffer {
    _bytes: Box<[u8]>,
}
impl AsScriptModuleUserData for PixelBuffer {}

// functions to export
#[aviutl2::module::functions]
impl VectorCanvasBackend {
    fn render(
        &self,
        w: i32,
        h: i32,
        commands: String,
    ) -> AnyResult<(*const u8, ScriptModuleUserData<PixelBuffer>)> {
        let rgba = render::render_to_rgba_buffer(w, h, &commands)?;
        let data = rgba.as_ptr();
        let owner = ScriptModuleUserData::new(PixelBuffer { _bytes: rgba });
        Ok((data, owner))
    }
}

// registration
aviutl2::register_script_module!(VectorCanvasBackend);
