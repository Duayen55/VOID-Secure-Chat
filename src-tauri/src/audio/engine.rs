use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

pub struct AudioEngine {
    stream: Option<cpal::Stream>,
}

impl AudioEngine {
    pub fn new() -> Self {
        Self { stream: None }
    }

    pub fn start(&mut self) -> Result<(), String> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or("No input device available")?;

        println!("Input device: {:?}", device.id());

        let config = device.default_input_config().map_err(|e| e.to_string())?;

        let stream = device
            .build_input_stream(
                &config.into(),
                move |data: &[f32], _: &_| {
                    // TODO: Send data to network
                    // Simple VAD placeholder
                    let rms = data.iter().map(|x| x * x).sum::<f32>() / data.len() as f32;
                    if rms > 0.01 {
                        // println!("Voice detected: rms={}", rms);
                    }
                },
                move |err| {
                    eprintln!("an error occurred on stream: {}", err);
                },
                None,
            )
            .map_err(|e| e.to_string())?;

        stream.play().map_err(|e| e.to_string())?;
        self.stream = Some(stream);

        Ok(())
    }

    pub fn stop(&mut self) {
        self.stream = None; // Dropping the stream stops it
    }
}
