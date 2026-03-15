pub struct FfmpegProfiles;

impl FfmpegProfiles {
    /// Профиль 1: Ультрабыстрая разведка путей (Таймаут 1 сек)
    pub fn probe(url: &str) -> Vec<String> {
        vec![
            "-nostdin".into(),
            "-loglevel".into(),
            "error".into(),
            "-rtsp_transport".into(),
            "tcp".into(),
            "-timeout".into(),
            "1000000".into(), // Быстрый отстрел неверных путей
            "-i".into(),
            url.to_string(),
            "-t".into(),
            "0.1".into(),
            "-f".into(),
            "null".into(),
            "-".into(),
        ]
    }

    /// Профиль 2: ЕДИНЫЙ ЗОЛОТОЙ СТАНДАРТ (1.5 МБ буфер, без nobuffer)
    pub fn web_stream(url: &str, extra_headers: Option<&str>) -> Vec<String> {
        let mut args = Vec::new();

        if let Some(headers) = extra_headers {
            args.push("-headers".into());
            args.push(headers.to_string());
        }

        let core_args = vec![
            "-rtsp_transport",
            "tcp",
            "-allowed_media_types",
            "video",
            "-timeout",
            "5000000",
            // ВАЖНО: Никакого nobuffer. Спасает от залипаний и артефактов на старте.
            "-fflags",
            "+genpts+discardcorrupt",
            "-flags",
            "low_delay",
            // Те самые 1.5 мегабайта для мгновенного сбора первого I-кадра
            "-analyzeduration",
            "1500000",
            "-probesize",
            "1500000",
            "-i",
            url,
            "-c:v",
            "libx264",
            "-preset",
            "ultrafast",
            "-tune",
            "zerolatency",
            "-profile:v",
            "baseline",
            "-pix_fmt",
            "yuv420p",
            "-g",
            "30",
            "-an",
            "-f",
            "flv",
            "-flvflags",
            "no_duration_filesize",
            "pipe:1",
        ];

        for arg in core_args {
            args.push(arg.to_string());
        }

        args
    }
}
