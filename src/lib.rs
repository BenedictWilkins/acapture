mod utils;
use numpy::ndarray::{Array, ArrayBase, Dim, OwnedRepr, ShapeBuilder};
use pyo3::exceptions::PyValueError;
use utils::error::WarnOnErr;

use std::sync::RwLock;

use numpy::{IntoPyArray, PyArray, PyArray1, PyArray3};
use pyo3::prelude::*;

use pyo3::types::{PyDict, PyList, PyTuple};
use scap::{
    capturer::{Capturer, Options},
    frame::Frame,
    Target,
};

type ImageArray = Array<u8, Dim<[usize; 3]>>;

struct SafeCapturer {
    inner: RwLock<Capturer>,
    closed: bool,
}

unsafe impl Send for SafeCapturer {}
unsafe impl Sync for SafeCapturer {}

impl SafeCapturer {
    /// Create a new capturer instance with the provided options
    pub fn new(options: Options) -> SafeCapturer {
        return SafeCapturer {
            inner: RwLock::new(Capturer::new(options)),
            closed: true,
        };
    }

    pub fn start_capture(&mut self) -> Result<(), String> {
        if self.closed {
            self.inner.write().unwrap().start_capture();
            self.closed = false;
            return Ok(());
        }
        return Err("Failed to start capture, it is already running.".to_string());
    }

    pub fn stop_capture(&mut self) -> Result<(), String> {
        if !self.closed {
            self.inner.write().unwrap().stop_capture();
            self.closed = true;
            return Ok(());
        }
        return Err("Failed to stop capture, it was not running.".to_string());
    }

    /// Get the next captured frame
    pub fn get_next_frame(&self) -> Result<Frame, String> {
        // First, attempt to lock the `inner` object with read access
        let capturer = self
            .inner
            .read()
            .map_err(|e| format!("Failed to acquire read lock: {}", e))?;
        // Then, attempt to get the next frame, converting the error if it occurs
        capturer
            .get_next_frame()
            .map_err(|e| format!("Failed to get next frame: {}", e))
    }

    /// Get the dimensions the frames will be captured in
    pub fn get_output_frame_size(&mut self) -> [u32; 2] {
        // does this need to be &mut ? does it modify options internally?
        return self.inner.write().unwrap().get_output_frame_size();
    }

    pub fn get_target_by_id(id: u32) -> Result<Target, String> {
        fn get_target_id(target: &Target) -> u32 {
            return match target {
                Target::Window(window) => window.id,
                Target::Display(display) => display.id,
            };
        }
        return scap::get_all_targets()
            .into_iter()
            .find(|target| get_target_id(target) == id)
            .ok_or_else(|| format!("Failed to find target with id: {}", id));
    }
}

#[pyclass]
struct Environment {
    capturer: SafeCapturer,
}

#[pyfunction]
fn get_targets<'py>(py: Python<'py>) -> pyo3::Bound<'_, PyList> {
    let targets: Vec<Target> = scap::get_all_targets();
    let targets_py: Vec<Bound<'py, PyTuple>> = targets
        .into_iter()
        .map(|target| match target {
            Target::Window(window) => {
                PyTuple::new_bound(py, &[window.id.into_py(py), window.title.into_py(py)])
            }
            Target::Display(display) => {
                PyTuple::new_bound(py, &[display.id.into_py(py), display.title.into_py(py)])
            }
        })
        .collect();
    return PyList::new_bound(py, targets_py);
}

#[pymethods]
impl Environment {
    #[new]
    fn new(
        py: Python<'_>,
        target_id: Option<u32>,
        show_cursor: Option<bool>,
        show_highlight: Option<bool>,
        fps: Option<u32>,
    ) -> Self {
        let target = target_id.map(SafeCapturer::get_target_by_id).transpose();
        let options = Options {
            fps: fps.unwrap_or(32),
            target: target.expect("Error"),
            show_cursor: show_cursor.unwrap_or(false),
            show_highlight: show_highlight.unwrap_or(true),
            excluded_targets: None,
            ..Default::default()
        };

        let mut env = Environment {
            capturer: SafeCapturer::new(options),
        };
        env.capturer.start_capture().unwrap_warn(py);
        return env;
    }

    fn reset<'py>(&mut self, py: Python<'py>) -> PyResult<Bound<'py, PyTuple>> {
        //if self.capturer.
        if !self.capturer.closed {
            self.capturer.stop_capture().unwrap_warn(py);
        }
        self.capturer.start_capture().unwrap_warn(py);

        let frame: Frame = self
            .capturer
            .get_next_frame()
            .map_err(|e| PyValueError::new_err(e))?;
        let image = self
            .get_image_from_frame(frame)
            .map_err(|e| PyValueError::new_err(e))?;
        let pyimage = image.into_pyarray_bound(py);
        let pyinfo = PyDict::new_bound(py); // empty dict for info
        return Ok(PyTuple::new_bound(
            py,
            vec![pyimage.to_object(py), pyinfo.to_object(py)],
        ));
    }

    fn close(&mut self, py: Python<'_>) {
        self.capturer.stop_capture().unwrap_warn(py);
    }

    // Method to query the current state as a NumPy array without copying
    // fn step<'py>(&self, py: Python<'py>) -> pyo3::Bound<'py, PyArray1<f64>> {
    //     /let observation = self.get_next_frame();
    //     return observation;
    // }
}

impl Environment {
    fn get_image_from_frame(&self, frame: Frame) -> Result<ImageArray, String> {
        match frame {
            Frame::BGRA(frame) => {
                // HWC shape (height, width, channels), the alpha channel will be skipped
                let shape = [frame.height as usize, frame.width as usize, 3 as usize];
                let strides = [
                    (frame.width * 4) as usize, // Moving across rows (advance by 4 bytes * width)
                    4 as usize,                 // Moving across columns (4 bytes per pixel in BGRA)
                    1 as usize,                 // Moving across channels (1 byte per channel)
                ];

                // Create an ArrayView3 with custom strides, slicing to get the first 3 channels (B, G, R)
                let image = Array::from_shape_vec(shape.strides(strides), frame.data)
                    .map_err(|e| format!("{}", e))?;
                Ok(image)
            }

            _ => Err(format!("Recived invalid frame type: {:?}", frame)),
        }

        // PyArray3::from_vec3_bound(py, v)
        // return PyArray1::from_slice_bound(py, &self.data); // Borrow the data without copying
    }
}

#[pymodule]
fn acapture(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Environment>()?;
    m.add_function(wrap_pyfunction!(get_targets, m)?)?;
    Ok(())
}

// #[pyfunction]
// fn capture<'py>(py: Python<'py>) -> pyo3::Bound<'_, PyArray1<f64>> {
//     // Create a Rust Vec<f64>
//     let vec: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0, 5.0];
//     // Convert the Vec<f64> to a NumPy array without copying the data
//     return PyArray1::from_vec_bound(py, vec);
// }
