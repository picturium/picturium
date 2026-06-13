use crate::enums::filter::FilterValue;
use crate::process::pipeline::request::PipelineRequest;
use anyhow::Result;
use picturium_libvips::{VipsFilters, VipsImage};

pub fn process(request: &PipelineRequest, image: VipsImage) -> Result<VipsImage> {
    let mut filter_queue = FilterQueue::new(image.get_bit_depth());
    println!("Filter source queue: {:?}", request.parameters.filter.0);

    request.parameters.filter.0.iter()
        .try_for_each(|filter| match filter {
            FilterValue::Brightness(value) => filter_queue.apply_brightness(*value),
            FilterValue::Contrast(value) => filter_queue.apply_contrast(*value),
            _ => Ok(()),
        })?;

    println!("Filter queue: {:?}", filter_queue);
    Ok(execute_filter_queue(&mut filter_queue, image)?)
}

#[derive(Debug)]
struct FilterQueue {
    linear: Option<(f64, f64)>,
    divider: f64,
}

impl FilterQueue {
    fn new(bit_depth: i32) -> Self {
        Self {
            linear: None,
            divider: (1 << bit_depth) as f64,
        }
    }

    fn linear_mut(&mut self) -> &mut (f64, f64) {
        self.linear.get_or_insert((1.0, 0.0))
    }

    pub fn apply_brightness(&mut self, value: f64) -> Result<()> {
        self.linear_mut().0 *= value;
        Ok(())
    }

    pub fn apply_contrast(&mut self, value: f64) -> Result<()> {
        let divider = self.divider;
        let linear = self.linear_mut();

        linear.0 *= value;
        linear.1 = linear.1 * value + (0.5 - value * 0.5) * divider;

        Ok(())
    }
}

fn execute_filter_queue(queue: &mut FilterQueue, image: VipsImage) -> Result<VipsImage> {
    let mut image = image;

    if let Some((a, b)) = queue.linear {
        image = image.linear(a, b)
            .map_err(|e| anyhow::anyhow!("Failed to apply linear filter: {:?}", e))?;
    }

    Ok(image)
}
