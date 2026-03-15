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

    /// Профиль 2: Умная трансляция с адаптацией под вендора (H.265 -> FLV)
    pub fn web_stream(url: &str, extra_headers: Option<&str>) -> Vec<String> {
        let mut args = Vec::new();

        if let Some(headers) = extra_headers {
            args.push("-headers".into());
            args.push(headers.to_string());
        }

        // Общие сетевые настройки захвата
        args.extend(vec![
            "-rtsp_transport".into(),
            "tcp".into(),
            "-allowed_media_types".into(),
            "video".into(),
            "-timeout".into(),
            "5000000".into(),
            "-flags".into(),
            "low_delay".into(),
        ]);

        // ДИНАМИЧЕСКИЙ ПРОФИЛЬ (Разделяем логику для Hikvision и Navicam/XMeye)
        let is_hikvision = url.to_lowercase().contains("/streaming/channels/");

        if is_hikvision {
            // Профиль Hikvision: требует времени на сборку редких I-кадров H.265
            args.extend(vec![
                "-fflags".into(),
                "+genpts+discardcorrupt".into(),
                "-analyzeduration".into(),
                "2000000".into(),
                "-probesize".into(),
                "2000000".into(),
            ]);
        } else {
            // Профиль Navicam / Tantos (XMeye): Давятся буфером, требуют nobuffer
            args.extend(vec![
                "-fflags".into(),
                "nobuffer+genpts+discardcorrupt".into(),
                "-analyzeduration".into(),
                "250000".into(), // Старт за 0.25 сек
                "-probesize".into(),
                "250000".into(),
            ]);
        }

        // Общие настройки сверхбыстрого транскодера
        let rest = vec![
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
            "-an", // Аудио отключаем жестко
            "-f",
            "flv",
            "-flvflags",
            "no_duration_filesize",
            "pipe:1",
        ];

        for arg in rest {
            args.push(arg.to_string());
        }

        args
    }
}
