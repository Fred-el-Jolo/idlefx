//! Redrawing the canvas at a stable size performs no heap allocation.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

use idlefx_core::{Canvas, Rgb};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

struct Counting;

static ALLOCS: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::SeqCst);
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::SeqCst);
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

fn frame(canvas: &mut Canvas, buf: &mut Buffer, area: Rect, t: f32) {
    canvas.fit(area);
    canvas.clear();
    canvas.ellipse(20.0 + t, 16.0, 9.0, 4.5, 0.2, Rgb(200, 150, 90));
    canvas.capsule(10.0, 20.0, 12.0 + t, 31.0, 1.2, Rgb(120, 80, 40));
    canvas.rect(0.0, 30.0, 64.0, 32.0, Rgb(40, 40, 40));
    canvas.pixel(30.0, 10.0, Rgb(0, 0, 0));
    canvas.blit(buf, area);
}

#[test]
fn redraw_at_stable_size_does_not_allocate() {
    let area = Rect::new(0, 0, 64, 16);
    let mut buf = Buffer::empty(area);
    let mut canvas = Canvas::default();
    // The first frame sizes the canvas.
    frame(&mut canvas, &mut buf, area, 0.0);

    let before = ALLOCS.load(Ordering::SeqCst);
    for i in 0..200 {
        frame(&mut canvas, &mut buf, area, i as f32 * 0.1);
    }
    let after = ALLOCS.load(Ordering::SeqCst);
    assert_eq!(after - before, 0, "allocations while redrawing");
}
