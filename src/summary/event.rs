use super::*;

/// A [Event] initializer.
#[derive(Debug, Clone, PartialEq)]
pub struct EventInit {
    /// The wall clock time in microseconds.
    ///
    /// If the field is set to `None`, it sets to current system time when the event is built.
    pub wall_time: Option<f64>,
    /// The global step.
    pub step: i64,
}

impl EventInit {
    /// Create a initializer with global step and wall time.
    pub fn new(step: i64, wall_time: f64) -> Self {
        Self {
            wall_time: Some(wall_time),
            step,
        }
    }

    /// Create a initializer with global step and without wall time.
    pub fn with_step(step: i64) -> Self {
        Self {
            wall_time: None,
            step,
        }
    }

    /// Build an empty event.
    pub fn build_empty(self) -> Event {
        let (wall_time, step) = self.to_parts();
        Event {
            wall_time,
            step,
            what: None,
        }
    }

    /// Build an event with summary.
    pub fn build_with_summary(self, summary: Summary) -> Event {
        let (wall_time, step) = self.to_parts();
        Event {
            wall_time,
            step,
            what: Some(What::Summary(summary)),
        }
    }

    fn to_parts(self) -> (f64, i64) {
        let Self {
            wall_time: wall_time_opt,
            step,
        } = self;
        let wall_time = wall_time_opt.unwrap_or_else(|| Self::get_wall_time());
        (wall_time, step)
    }

    fn get_wall_time() -> f64 {
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as f64
            / 1.0e9
    }
}

impl From<i64> for EventInit {
    fn from(step: i64) -> Self {
        Self::with_step(step)
    }
}

impl From<(i64, f64)> for EventInit {
    fn from((step, wall_time): (i64, f64)) -> Self {
        Self::new(step, wall_time)
    }
}

impl From<(i64, SystemTime)> for EventInit {
    fn from((step, time): (i64, SystemTime)) -> Self {
        Self::new(
            step,
            time.duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as f64
                / 1.0e9,
        )
    }
}

/// A [Summary] initializer.
#[derive(Debug, Clone, PartialEq)]
pub struct SummaryInit<T>
where
    T: ToString,
{
    /// The tag of the summary.
    pub tag: T,
}

impl<T> SummaryInit<T>
where
    T: ToString,
{
    /// Create an initializer with a tag.
    pub fn new(tag: T) -> Self {
        Self { tag }
    }

    /// Build a scalar summary.
    pub fn build_scalar(self, value: f32) -> Result<Summary, Error> {
        let Self { tag } = self;

        let summary = Summary {
            value: vec![Value {
                node_name: "".into(),
                tag: tag.to_string(),
                metadata: None,
                value: Some(ValueEnum::SimpleValue(value)),
            }],
        };
        Ok(summary)
    }

    /// Build a string summary.
    pub fn build_string<S: ToString>(self, value: S) -> Result<Summary, Error> {
        let Self { tag } = self;

        let v = value.to_string();
        let summary = Summary {
            value: vec![Value {
                node_name: "".into(),
                tag: tag.to_string(),
                metadata: Some(SummaryMetadata {
                    plugin_data: Some(PluginData {
                        plugin_name: "text".into(),
                        content: vec![],
                    }),
                    display_name: "".to_string(),
                    summary_description: v.to_string(),
                    data_class: 0,
                }),
                value: Some(ValueEnum::Tensor(TensorProto {
                    dtype: DataType::DtString as i32,
                    tensor_shape: TensorShapeProto {
                        dim: vec![Dim {
                            size: 1,
                            name: "".into(),
                        }],
                        unknown_rank: false,
                    },
                    version_number: 0,
                    string_val: vec![v.into_bytes()],
                    ..Default::default()
                })),
            }],
        };
        Ok(summary)
    }

    /// Build a histogram summary.
    pub fn build_histogram<H, E>(self, histogram: H) -> Result<Summary, Error>
    where
        H: TryInto<HistogramProto, Error = E>,
        Error: From<E>,
    {
        let Self { tag } = self;

        let summary = Summary {
            value: vec![Value {
                node_name: "".into(),
                tag: tag.to_string(),
                metadata: None,
                value: Some(ValueEnum::Histo(histogram.try_into()?)),
            }],
        };
        Ok(summary)
    }

    /// Build a tensor summary.
    pub fn build_tensor<S, E>(self, tensor: S) -> Result<Summary, Error>
    where
        S: TryInto<TensorProto, Error = E>,
        Error: From<E>,
    {
        let Self { tag } = self;

        let summary = Summary {
            value: vec![Value {
                node_name: "".into(),
                tag: tag.to_string(),
                metadata: None,
                value: Some(ValueEnum::Tensor(tensor.try_into()?)),
            }],
        };
        Ok(summary)
    }

    /// Build an image summary.
    pub fn build_image<M, E>(self, image: M) -> Result<Summary, Error>
    where
        M: TryInto<Image, Error = E>,
        Error: From<E>,
    {
        let Self { tag } = self;

        let summary = Summary {
            value: vec![Value {
                node_name: "".into(),
                tag: tag.to_string(),
                metadata: None,
                value: Some(ValueEnum::Image(image.try_into()?)),
            }],
        };
        Ok(summary)
    }

    /// Build a summary with multiple images.
    pub fn build_image_list<V, E>(self, images: V) -> Result<Summary, Error>
    where
        V: TryInfoImageList<Error = E>,
        Error: From<E>,
    {
        let Self { tag } = self;

        let image_protos = images.try_into_image_list()?;

        let values = match image_protos.len() {
            1 => {
                let image_proto = image_protos.into_iter().next().unwrap();
                let values = vec![Value {
                    node_name: "".into(),
                    tag: format!("{}/image", tag.to_string()),
                    metadata: None,
                    value: Some(ValueEnum::Image(image_proto)),
                }];
                values
            }
            _ => {
                let values = image_protos
                    .into_iter()
                    .enumerate()
                    .map(|(index, image_proto)| Value {
                        node_name: "".into(),
                        tag: format!("{}/image/{}", tag.to_string(), index),
                        metadata: None,
                        value: Some(ValueEnum::Image(image_proto)),
                    })
                    .collect::<Vec<_>>();
                values
            }
        };

        let summary = Summary { value: values };
        Ok(summary)
    }

    /// Build an audio summary.
    pub fn build_audio<A, E>(self, audio: A) -> Result<Summary, Error>
    where
        A: TryInto<Audio, Error = E>,
        Error: From<E>,
    {
        let Self { tag } = self;

        let summary = Summary {
            value: vec![Value {
                node_name: "".into(),
                tag: tag.to_string(),
                metadata: None,
                value: Some(ValueEnum::Audio(audio.try_into()?)),
            }],
        };
        Ok(summary)
    }
}
