use core::fmt;
use std::sync::{Arc, OnceLock};

use std::future::Future;

/// Convert a color from colorgrad to egui's Color32
pub fn colorgrad_to_egui(color: &colorgrad::Color) -> egui::Color32 {
    let [r, g, b, a] = color.to_rgba8();
    egui::Color32::from_rgba_unmultiplied(r, g, b, a)
}

pub fn oklab(l: f32, a: f32, b: f32, alpha: f32) -> egui::Color32 {
    colorgrad_to_egui(&colorgrad::Color::from_oklaba(l, a, b, alpha))
}

pub struct FutureData<T> {
    data: Arc<OnceLock<T>>,
}

impl<T: fmt::Debug + 'static> FutureData<T> {
    pub fn spawn(f: impl Future<Output = T> + 'static) -> Self {
        let data = Arc::new(OnceLock::new());
        let data2 = data.clone();
        wasm_bindgen_futures::spawn_local(async move {
            data2.set(f.await).unwrap();
        });
        Self { data }
    }

    pub fn try_get(&self) -> Option<&T> {
        self.data.get()
    }
}
