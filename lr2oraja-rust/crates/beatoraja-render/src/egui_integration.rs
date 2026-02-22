// egui integration — manages egui::Context and egui_wgpu::Renderer for overlay UI

/// Manages the egui rendering pipeline on top of wgpu.
///
/// Java equivalent: ImGuiImplGl3 + ImGuiImplGlfw (imgui-java OpenGL backend).
/// Rust replacement: egui_wgpu::Renderer handles GPU-side tessellation and rendering.
pub struct EguiIntegration {
    pub ctx: egui::Context,
    renderer: egui_wgpu::Renderer,
}

impl EguiIntegration {
    /// Create a new egui integration bound to the given wgpu device and surface format.
    ///
    /// Java equivalent: ImGui.createContext() + imGuiGl3.init("#version 150")
    pub fn new(device: &wgpu::Device, output_format: wgpu::TextureFormat) -> Self {
        let ctx = egui::Context::default();
        let renderer = egui_wgpu::Renderer::new(device, output_format, None, 1, false);
        Self { ctx, renderer }
    }

    /// Render egui output into the given wgpu command encoder.
    ///
    /// Java equivalent: ImGui.render() + imGuiGl3.renderDrawData(ImGui.getDrawData())
    ///
    /// This should be called after the main render pass (LoadOp::Clear) so that egui
    /// overlays on top of the game scene. Uses LoadOp::Load to preserve existing content.
    pub fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        screen_descriptor: &egui_wgpu::ScreenDescriptor,
        full_output: egui::FullOutput,
    ) {
        let clipped_primitives = self
            .ctx
            .tessellate(full_output.shapes, full_output.pixels_per_point);

        // Upload changed textures
        for (id, image_delta) in &full_output.textures_delta.set {
            self.renderer
                .update_texture(device, queue, *id, image_delta);
        }

        // Update vertex/index buffers
        self.renderer.update_buffers(
            device,
            queue,
            encoder,
            &clipped_primitives,
            screen_descriptor,
        );

        // Create render pass with LoadOp::Load to preserve the game scene,
        // then convert to 'static lifetime for egui_wgpu compatibility (wgpu 24).
        let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("egui render pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        // wgpu 24: RenderPass::forget_lifetime() converts RenderPass<'encoder> → RenderPass<'static>
        // Required by egui_wgpu::Renderer::render() which expects RenderPass<'static>
        let mut render_pass = render_pass.forget_lifetime();

        self.renderer
            .render(&mut render_pass, &clipped_primitives, screen_descriptor);
        drop(render_pass);

        // Free textures no longer needed
        for id in &full_output.textures_delta.free {
            self.renderer.free_texture(id);
        }
    }

    /// Get a reference to the egui context.
    pub fn context(&self) -> &egui::Context {
        &self.ctx
    }
}
