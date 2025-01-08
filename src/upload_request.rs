use crate::content_type::ContentType;

pub struct UploadRequest {
    pub string_content: String,
    pub filename: String,
    pub content_type: ContentType,
    pub human_readable_size: String,
    pub author: String,
}