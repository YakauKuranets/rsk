pub struct FfmpegProfiles;

impl FfmpegProfiles {
    /// Профиль 1: Ультрабыстрая разведка путей (Автопилот)
    pub fn probe(url: &str) -> Vec<String> {
        vec![
            "-nostdin".into(),
            "-loglevel".into(),
            "error".into(),
            "-rtsp_transport".into(),
            "tcp".into(),
            "-timeout".into(),
            "1000000".into(), // Жесткий таймаут 1 сек
            "-i".into(),
            url.to_string(),
            "-t".into(),
            "0.1".into(),
            "-f".into(),
            "null".into(),
            "-".into(),
        ]
    }

    /// Профиль 2: Zero-Latency трансляция (H.265 -> FLV)
    pub fn web_stream(url: &str, extra_headers: Option<&str>) -> Vec<String> {
        let mut args = Vec::new();

        // Встраиваем кастомные заголовки (например, Cookie для Hub), если они переданы
        if let Some(headers) = extra_headers {
            args.push("-headers".into());
            args.push(headers.to_string());
        }

        // Бронебойное ядро настроек для китайских камер
        let core_args = vec![
            "-rtsp_transport",
            "tcp",
            "-allowed_media_types",
            "video",
            "-timeout",
            "5000000",
            "-fflags",
            "+genpts+discardcorrupt",
            "-flags",
            "low_delay",
            "-analyzeduration",
            "2000000",
            "-probesize",
            "2000000",
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
