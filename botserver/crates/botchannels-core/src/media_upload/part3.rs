use super::*;

impl MediaUploadService {
    pub fn new() -> Self {
        let mut platform_limits = HashMap::new();

        platform_limits.insert(
            Platform::Twitter,
            PlatformLimits {
                max_image_size_bytes: 5 * 1024 * 1024,
                max_video_size_bytes: 512 * 1024 * 1024,
                max_video_duration_seconds: 140,
                supported_image_formats: vec!["jpg".into(), "jpeg".into(), "png".into(), "gif".into(), "webp".into()],
                supported_video_formats: vec!["mp4".into(), "mov".into()],
                max_images_per_post: 4,
                max_videos_per_post: 1,
                image_dimensions: Some(ImageDimensions {
                    min_width: 4,
                    min_height: 4,
                    max_width: 8192,
                    max_height: 8192,
                    recommended_width: 1200,
                    recommended_height: 675,
                }),
                video_dimensions: Some(VideoDimensions {
                    min_width: 32,
                    min_height: 32,
                    max_width: 1920,
                    max_height: 1200,
                    min_fps: 15,
                    max_fps: 60,
                    min_bitrate_kbps: 100,
                    max_bitrate_kbps: 25000,
                }),
                requires_aspect_ratio: None,
            },
        );

        platform_limits.insert(
            Platform::Instagram,
            PlatformLimits {
                max_image_size_bytes: 8 * 1024 * 1024,
                max_video_size_bytes: 4 * 1024 * 1024 * 1024,
                max_video_duration_seconds: 3600,
                supported_image_formats: vec!["jpg".into(), "jpeg".into(), "png".into()],
                supported_video_formats: vec!["mp4".into(), "mov".into()],
                max_images_per_post: 10,
                max_videos_per_post: 1,
                image_dimensions: Some(ImageDimensions {
                    min_width: 320,
                    min_height: 320,
                    max_width: 1080,
                    max_height: 1350,
                    recommended_width: 1080,
                    recommended_height: 1080,
                }),
                video_dimensions: Some(VideoDimensions {
                    min_width: 500,
                    min_height: 500,
                    max_width: 1920,
                    max_height: 1080,
                    min_fps: 23,
                    max_fps: 60,
                    min_bitrate_kbps: 1000,
                    max_bitrate_kbps: 25000,
                }),
                requires_aspect_ratio: Some("4:5 to 1.91:1".into()),
            },
        );

        platform_limits.insert(
            Platform::TikTok,
            PlatformLimits {
                max_image_size_bytes: 20 * 1024 * 1024,
                max_video_size_bytes: 4 * 1024 * 1024 * 1024,
                max_video_duration_seconds: 600,
                supported_image_formats: vec!["jpg".into(), "jpeg".into(), "png".into(), "webp".into()],
                supported_video_formats: vec!["mp4".into(), "webm".into(), "mov".into()],
                max_images_per_post: 35,
                max_videos_per_post: 1,
                image_dimensions: None,
                video_dimensions: Some(VideoDimensions {
                    min_width: 720,
                    min_height: 1280,
                    max_width: 1080,
                    max_height: 1920,
                    min_fps: 24,
                    max_fps: 60,
                    min_bitrate_kbps: 1000,
                    max_bitrate_kbps: 50000,
                }),
                requires_aspect_ratio: Some("9:16".into()),
            },
        );

        platform_limits.insert(
            Platform::YouTube,
            PlatformLimits {
                max_image_size_bytes: 2 * 1024 * 1024,
                max_video_size_bytes: 256 * 1024 * 1024 * 1024,
                max_video_duration_seconds: 43200,
                supported_image_formats: vec!["jpg".into(), "jpeg".into(), "png".into()],
                supported_video_formats: vec![
                    "mp4".into(), "mov".into(), "avi".into(), "wmv".into(),
                    "flv".into(), "webm".into(), "mkv".into(), "3gp".into(),
                ],
                max_images_per_post: 1,
                max_videos_per_post: 1,
                image_dimensions: Some(ImageDimensions {
                    min_width: 640,
                    min_height: 360,
                    max_width: 2560,
                    max_height: 1440,
                    recommended_width: 1280,
                    recommended_height: 720,
                }),
                video_dimensions: Some(VideoDimensions {
                    min_width: 426,
                    min_height: 240,
                    max_width: 7680,
                    max_height: 4320,
                    min_fps: 24,
                    max_fps: 60,
                    min_bitrate_kbps: 500,
                    max_bitrate_kbps: 128000,
                }),
                requires_aspect_ratio: Some("16:9".into()),
            },
        );

        platform_limits.insert(
            Platform::LinkedIn,
            PlatformLimits {
                max_image_size_bytes: 8 * 1024 * 1024,
                max_video_size_bytes: 5 * 1024 * 1024 * 1024,
                max_video_duration_seconds: 600,
                supported_image_formats: vec!["jpg".into(), "jpeg".into(), "png".into(), "gif".into()],
                supported_video_formats: vec!["mp4".into(), "mov".into(), "avi".into()],
                max_images_per_post: 9,
                max_videos_per_post: 1,
                image_dimensions: Some(ImageDimensions {
                    min_width: 552,
                    min_height: 276,
                    max_width: 7680,
                    max_height: 4320,
                    recommended_width: 1200,
                    recommended_height: 627,
                }),
                video_dimensions: Some(VideoDimensions {
                    min_width: 256,
                    min_height: 144,
                    max_width: 4096,
                    max_height: 2304,
                    min_fps: 15,
                    max_fps: 60,
                    min_bitrate_kbps: 500,
                    max_bitrate_kbps: 30000,
                }),
                requires_aspect_ratio: None,
            },
        );

        platform_limits.insert(
            Platform::Facebook,
            PlatformLimits {
                max_image_size_bytes: 10 * 1024 * 1024,
                max_video_size_bytes: 10 * 1024 * 1024 * 1024,
                max_video_duration_seconds: 14400,
                supported_image_formats: vec!["jpg".into(), "jpeg".into(), "png".into(), "gif".into(), "bmp".into(), "tiff".into()],
                supported_video_formats: vec!["mp4".into(), "mov".into(), "avi".into(), "wmv".into(), "mkv".into()],
                max_images_per_post: 10,
                max_videos_per_post: 1,
                image_dimensions: Some(ImageDimensions {
                    min_width: 320,
                    min_height: 320,
                    max_width: 4096,
                    max_height: 4096,
                    recommended_width: 1200,
                    recommended_height: 630,
                }),
                video_dimensions: Some(VideoDimensions {
                    min_width: 120,
                    min_height: 120,
                    max_width: 4096,
                    max_height: 4096,
                    min_fps: 15,
                    max_fps: 60,
                    min_bitrate_kbps: 500,
                    max_bitrate_kbps: 50000,
                }),
                requires_aspect_ratio: None,
            },
        );

        platform_limits.insert(
            Platform::Pinterest,
            PlatformLimits {
                max_image_size_bytes: 32 * 1024 * 1024,
                max_video_size_bytes: 2 * 1024 * 1024 * 1024,
                max_video_duration_seconds: 3600,
                supported_image_formats: vec!["jpg".into(), "jpeg".into(), "png".into(), "gif".into(), "webp".into()],
                supported_video_formats: vec!["mp4".into(), "mov".into()],
                max_images_per_post: 5,
                max_videos_per_post: 1,
                image_dimensions: Some(ImageDimensions {
                    min_width: 100,
                    min_height: 100,
                    max_width: 10000,
                    max_height: 10000,
                    recommended_width: 1000,
                    recommended_height: 1500,
                }),
                video_dimensions: Some(VideoDimensions {
                    min_width: 240,
                    min_height: 240,
                    max_width: 1920,
                    max_height: 1080,
                    min_fps: 15,
                    max_fps: 60,
                    min_bitrate_kbps: 1000,
                    max_bitrate_kbps: 25000,
                }),
                requires_aspect_ratio: Some("2:3 recommended".into()),
            },
        );

        platform_limits.insert(
            Platform::Discord,
            PlatformLimits {
                max_image_size_bytes: 25 * 1024 * 1024,
                max_video_size_bytes: 500 * 1024 * 1024,
                max_video_duration_seconds: 0,
                supported_image_formats: vec!["jpg".into(), "jpeg".into(), "png".into(), "gif".into(), "webp".into()],
                supported_video_formats: vec!["mp4".into(), "webm".into(), "mov".into()],
                max_images_per_post: 10,
                max_videos_per_post: 10,
                image_dimensions: None,
                video_dimensions: None,
                requires_aspect_ratio: None,
            },
        );

        platform_limits.insert(
            Platform::Bluesky,
            PlatformLimits {
                max_image_size_bytes: 1024 * 1024,
                max_video_size_bytes: 50 * 1024 * 1024,
                max_video_duration_seconds: 60,
                supported_image_formats: vec!["jpg".into(), "jpeg".into(), "png".into()],
                supported_video_formats: vec!["mp4".into()],
                max_images_per_post: 4,
                max_videos_per_post: 1,
                image_dimensions: Some(ImageDimensions {
                    min_width: 100,
                    min_height: 100,
                    max_width: 2000,
                    max_height: 2000,
                    recommended_width: 1000,
                    recommended_height: 1000,
                }),
                video_dimensions: None,
                requires_aspect_ratio: None,
            },
        );

        platform_limits.insert(
            Platform::Threads,
            PlatformLimits {
                max_image_size_bytes: 8 * 1024 * 1024,
                max_video_size_bytes: 1024 * 1024 * 1024,
                max_video_duration_seconds: 300,
                supported_image_formats: vec!["jpg".into(), "jpeg".into(), "png".into()],
                supported_video_formats: vec!["mp4".into(), "mov".into()],
                max_images_per_post: 10,
                max_videos_per_post: 1,
                image_dimensions: Some(ImageDimensions {
                    min_width: 320,
                    min_height: 320,
                    max_width: 1440,
                    max_height: 1800,
                    recommended_width: 1080,
                    recommended_height: 1350,
                }),
                video_dimensions: Some(VideoDimensions {
                    min_width: 500,
                    min_height: 500,
                    max_width: 1920,
                    max_height: 1080,
                    min_fps: 23,
                    max_fps: 60,
                    min_bitrate_kbps: 1000,
                    max_bitrate_kbps: 25000,
                }),
                requires_aspect_ratio: None,
            },
        );

        platform_limits.insert(
            Platform::Snapchat,
            PlatformLimits {
                max_image_size_bytes: 5 * 1024 * 1024,
                max_video_size_bytes: 1024 * 1024 * 1024,
                max_video_duration_seconds: 180,
                supported_image_formats: vec!["jpg".into(), "jpeg".into(), "png".into()],
                supported_video_formats: vec!["mp4".into(), "mov".into()],
                max_images_per_post: 1,
                max_videos_per_post: 1,
                image_dimensions: Some(ImageDimensions {
                    min_width: 1080,
                    min_height: 1920,
                    max_width: 1080,
                    max_height: 1920,
                    recommended_width: 1080,
                    recommended_height: 1920,
                }),
                video_dimensions: Some(VideoDimensions {
                    min_width: 1080,
                    min_height: 1920,
                    max_width: 1080,
                    max_height: 1920,
                    min_fps: 24,
                    max_fps: 30,
                    min_bitrate_kbps: 1000,
                    max_bitrate_kbps: 8000,
                }),
                requires_aspect_ratio: Some("9:16".into()),
            },
        );

        platform_limits.insert(
            Platform::WeChat,
            PlatformLimits {
                max_image_size_bytes: 10 * 1024 * 1024,
                max_video_size_bytes: 200 * 1024 * 1024,
                max_video_duration_seconds: 900,
                supported_image_formats: vec!["jpg".into(), "jpeg".into(), "png".into(), "gif".into()],
                supported_video_formats: vec!["mp4".into()],
                max_images_per_post: 9,
                max_videos_per_post: 1,
                image_dimensions: None,
                video_dimensions: None,
                requires_aspect_ratio: None,
            },
        );

        platform_limits.insert(
            Platform::Reddit,
            PlatformLimits {
                max_image_size_bytes: 20 * 1024 * 1024,
                max_video_size_bytes: 1024 * 1024 * 1024,
                max_video_duration_seconds: 900,
                supported_image_formats: vec!["jpg".into(), "jpeg".into(), "png".into(), "gif".into()],
                supported_video_formats: vec!["mp4".into(), "mov".into()],
                max_images_per_post: 20,
                max_videos_per_post: 1,
                image_dimensions: None,
                video_dimensions: Some(VideoDimensions {
                    min_width: 480,
                    min_height: 270,
                    max_width: 1920,
                    max_height: 1080,
                    min_fps: 15,
                    max_fps: 60,
                    min_bitrate_kbps: 500,
                    max_bitrate_kbps: 20000,
                }),
                requires_aspect_ratio: None,
            },
        );

        Self {
            uploads: Arc::new(RwLock::new(HashMap::new())),
            chunked_uploads: Arc::new(RwLock::new(HashMap::new())),
            platform_limits,
        }
    }
}
