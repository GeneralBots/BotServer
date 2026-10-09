use super::*;

impl std::fmt::Display for TranscriptionProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TranscriptionProvider::Whisper => write!(f, "whisper"),
            TranscriptionProvider::AzureSpeech => write!(f, "azure_speech"),
            TranscriptionProvider::GoogleSpeech => write!(f, "google_speech"),
            TranscriptionProvider::AwsTranscribe => write!(f, "aws_transcribe"),
            TranscriptionProvider::DeepGram => write!(f, "deepgram"),
            TranscriptionProvider::AssemblyAI => write!(f, "assembly_ai"),
        }
    }
}
