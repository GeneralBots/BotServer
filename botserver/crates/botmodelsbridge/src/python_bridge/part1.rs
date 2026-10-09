use super::*;

impl PythonModel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::MediaPipe => "mediapipe",
            Self::DeepFace => "deepface",
            Self::FaceRecognition => "face_recognition",
            Self::InsightFace => "insightface",
            Self::Dlib => "dlib",
            Self::OpenCV => "opencv",
        }
    }
}
impl std::str::FromStr for PythonModel {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "mediapipe" => Ok(Self::MediaPipe),
            "deepface" => Ok(Self::DeepFace),
            "face_recognition" => Ok(Self::FaceRecognition),
            "insightface" => Ok(Self::InsightFace),
            "dlib" => Ok(Self::Dlib),
            "opencv" => Ok(Self::OpenCV),
            _ => Err(()),
        }
    }
}
impl PythonModel {
}
