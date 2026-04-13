use crate::{Bounds, DisplayId, Pixels, PlatformDisplay, Point, Size, px};
use anyhow::Result;
use parking_lot::Mutex;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone, Copy, Debug)]
pub struct IosDisplayMetrics {
    pub bounds: Bounds<Pixels>,
    pub visible_bounds: Bounds<Pixels>,
    pub scale_factor: f32,
}

impl Default for IosDisplayMetrics {
    fn default() -> Self {
        let bounds = Bounds {
            origin: Point::default(),
            size: Size {
                width: px(390.0),
                height: px(844.0),
            },
        };

        Self {
            bounds,
            visible_bounds: bounds,
            scale_factor: 3.0,
        }
    }
}

#[derive(Debug)]
pub(crate) struct IosDisplay {
    id: DisplayId,
    uuid: Uuid,
    metrics: Arc<Mutex<IosDisplayMetrics>>,
}

impl IosDisplay {
    pub(crate) fn new() -> Self {
        Self {
            id: DisplayId::new(1),
            uuid: Uuid::new_v4(),
            metrics: Arc::new(Mutex::new(IosDisplayMetrics::default())),
        }
    }

    pub(crate) fn metrics(&self) -> IosDisplayMetrics {
        *self.metrics.lock()
    }

    #[allow(dead_code)]
    pub(crate) fn update_metrics(&self, metrics: IosDisplayMetrics) {
        *self.metrics.lock() = metrics;
    }
}

impl PlatformDisplay for IosDisplay {
    fn id(&self) -> DisplayId {
        self.id
    }

    fn uuid(&self) -> Result<Uuid> {
        Ok(self.uuid)
    }

    fn bounds(&self) -> Bounds<Pixels> {
        self.metrics().bounds
    }

    fn visible_bounds(&self) -> Bounds<Pixels> {
        self.metrics().visible_bounds
    }
}
