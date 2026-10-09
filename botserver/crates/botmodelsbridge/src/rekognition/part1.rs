use super::*;

#[derive(Debug, Clone)]
pub enum RekognitionError {
    ConfigError(String),
    AwsError(String),
    InvalidImage(String),
    FaceNotFound(String),
    CollectionNotFound(String),
    CollectionAlreadyExists(String),
    QuotaExceeded,
    ServiceUnavailable,
    Unauthorized,
}
impl std::fmt::Display for RekognitionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConfigError(s) => write!(f, "Config error: {s}"),
            Self::AwsError(s) => write!(f, "AWS error: {s}"),
            Self::InvalidImage(s) => write!(f, "Invalid image: {s}"),
            Self::FaceNotFound(s) => write!(f, "Face not found: {s}"),
            Self::CollectionNotFound(s) => write!(f, "Collection not found: {s}"),
            Self::CollectionAlreadyExists(s) => write!(f, "Collection already exists: {s}"),
            Self::QuotaExceeded => write!(f, "Quota exceeded"),
            Self::ServiceUnavailable => write!(f, "Service unavailable"),
            Self::Unauthorized => write!(f, "Unauthorized"),
        }
    }
}
impl std::error::Error for RekognitionError {}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RekognitionConfig {
    pub region: String,
    pub access_key_id: Option<String>,
    pub secret_access_key: Option<String>,
    pub collection_id: Option<String>,
    pub max_faces: u32,
    pub quality_filter: QualityFilter,
    pub face_match_threshold: f32,
    pub use_face_liveness: bool,
    pub enable_age_range: bool,
    pub enable_emotions: bool,
    pub enable_smile_detection: bool,
    pub enable_eyeglasses_detection: bool,
    pub enable_sunglasses_detection: bool,
    pub enable_gender_detection: bool,
    pub enable_beard_detection: bool,
    pub enable_mustache_detection: bool,
    pub enable_eyes_open_detection: bool,
    pub enable_mouth_open_detection: bool,
    pub enable_face_occluded_detection: bool,
}
impl Default for RekognitionConfig {
    fn default() -> Self {
        Self {
            region: "us-east-1".to_string(),
            access_key_id: None,
            secret_access_key: None,
            collection_id: None,
            max_faces: 100,
            quality_filter: QualityFilter::Auto,
            face_match_threshold: 80.0,
            use_face_liveness: false,
            enable_age_range: true,
            enable_emotions: true,
            enable_smile_detection: true,
            enable_eyeglasses_detection: true,
            enable_sunglasses_detection: true,
            enable_gender_detection: true,
            enable_beard_detection: true,
            enable_mustache_detection: true,
            enable_eyes_open_detection: true,
            enable_mouth_open_detection: true,
            enable_face_occluded_detection: true,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum QualityFilter {
    None,
    Auto,
    Low,
    Medium,
    High,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RekognitionFace {
    pub face_id: Option<String>,
    pub bounding_box: BoundingBox,
    pub confidence: f32,
    pub image_id: Option<String>,
    pub external_image_id: Option<String>,
    pub landmarks: Vec<Landmark>,
    pub pose: Pose,
    pub quality: FaceQuality,
    pub age_range: Option<AgeRange>,
    pub smile: Option<Smile>,
    pub eyeglasses: Option<Eyeglasses>,
    pub sunglasses: Option<Sunglasses>,
    pub gender: Option<Gender>,
    pub beard: Option<Beard>,
    pub mustache: Option<Mustache>,
    pub eyes_open: Option<EyesOpen>,
    pub mouth_open: Option<MouthOpen>,
    pub emotions: Vec<Emotion>,
    pub face_occluded: Option<FaceOccluded>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundingBox {
    pub width: f32,
    pub height: f32,
    pub left: f32,
    pub top: f32,
}
impl BoundingBox {
    pub fn to_absolute(&self, image_width: u32, image_height: u32) -> AbsoluteBoundingBox {
        AbsoluteBoundingBox {
            x: (self.left * image_width as f32) as u32,
            y: (self.top * image_height as f32) as u32,
            width: (self.width * image_width as f32) as u32,
            height: (self.height * image_height as f32) as u32,
        }
    }

    pub fn area(&self) -> f32 {
        self.width * self.height
    }

    pub fn center(&self) -> (f32, f32) {
        (self.left + self.width / 2.0, self.top + self.height / 2.0)
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbsoluteBoundingBox {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Landmark {
    pub landmark_type: LandmarkType,
    pub x: f32,
    pub y: f32,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum LandmarkType {
    EyeLeft,
    EyeRight,
    MouthLeft,
    MouthRight,
    Nose,
    LeftEyeBrowLeft,
    LeftEyeBrowRight,
    LeftEyeBrowUp,
    RightEyeBrowLeft,
    RightEyeBrowRight,
    RightEyeBrowUp,
    LeftEyeLeft,
    LeftEyeRight,
    LeftEyeUp,
    LeftEyeDown,
    RightEyeLeft,
    RightEyeRight,
    RightEyeUp,
    RightEyeDown,
    NoseLeft,
    NoseRight,
    MouthUp,
    MouthDown,
    LeftPupil,
    RightPupil,
    UpperJawlineLeft,
    MidJawlineLeft,
    ChinBottom,
    MidJawlineRight,
    UpperJawlineRight,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pose {
    pub roll: f32,
    pub yaw: f32,
    pub pitch: f32,
}
impl Pose {
    pub fn is_frontal(&self, threshold: f32) -> bool {
        self.yaw.abs() < threshold && self.pitch.abs() < threshold && self.roll.abs() < threshold
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaceQuality {
    pub brightness: f32,
    pub sharpness: f32,
}
impl FaceQuality {
    pub fn is_acceptable(&self, min_brightness: f32, min_sharpness: f32) -> bool {
        self.brightness >= min_brightness && self.sharpness >= min_sharpness
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgeRange {
    pub low: u32,
    pub high: u32,
}
impl AgeRange {
    pub fn midpoint(&self) -> u32 {
        (self.low + self.high) / 2
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Smile {
    pub value: bool,
    pub confidence: f32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Eyeglasses {
    pub value: bool,
    pub confidence: f32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sunglasses {
    pub value: bool,
    pub confidence: f32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gender {
    pub value: GenderValue,
    pub confidence: f32,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GenderValue {
    Male,
    Female,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Beard {
    pub value: bool,
    pub confidence: f32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mustache {
    pub value: bool,
    pub confidence: f32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EyesOpen {
    pub value: bool,
    pub confidence: f32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MouthOpen {
    pub value: bool,
    pub confidence: f32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaceOccluded {
    pub value: bool,
    pub confidence: f32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Emotion {
    pub emotion_type: EmotionType,
    pub confidence: f32,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EmotionType {
    Happy,
    Sad,
    Angry,
    Confused,
    Disgusted,
    Surprised,
    Calm,
    Fear,
    Unknown,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaceCollection {
    pub collection_id: String,
    pub collection_arn: Option<String>,
    pub face_count: u64,
    pub face_model_version: String,
    pub creation_timestamp: Option<DateTime<Utc>>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedFace {
    pub face_id: String,
    pub bounding_box: BoundingBox,
    pub image_id: String,
    pub external_image_id: Option<String>,
    pub confidence: f32,
}
impl Default for RekognitionService {
    fn default() -> Self {
        Self::new()
    }
}
