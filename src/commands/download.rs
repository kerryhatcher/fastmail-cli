use crate::jmap::authenticated_client;
use crate::models::Output;
use crate::util::{extract_text, infer_image_mime, is_image, parse_size, resize_image};
use std::ffi::OsStr;
use std::path::Path;

/// Strip any directory components from a server-supplied filename.
///
/// Prevents path traversal attacks (e.g. `../../../etc/passwd` → `passwd`).
/// Falls back to `"attachment"` when the path has no basename (e.g. `..` or empty string).
fn safe_filename(raw: &str) -> String {
    Path::new(raw)
        .file_name()
        .unwrap_or_else(|| OsStr::new("attachment"))
        .to_string_lossy()
        .to_string()
}

pub async fn download_attachment(
    email_id: &str,
    output_dir: Option<&str>,
    format: Option<&str>,
    max_size: Option<&str>,
) -> anyhow::Result<()> {
    let max_bytes = max_size.and_then(parse_size);
    let client = authenticated_client().await?;

    let email = client.get_email(email_id).await?;

    let Some(attachments) = &email.attachments else {
        Output::<()>::error("No attachments found").print();
        return Ok(());
    };
    if attachments.is_empty() {
        Output::<()>::error("No attachments found").print();
        return Ok(());
    }

    // JSON format - extract text and return structured data
    if format == Some("json") {
        let mut results: Vec<AttachmentContent> = Vec::new();

        for attachment in attachments {
            let blob_id = match &attachment.blob_id {
                Some(id) => id,
                None => continue,
            };

            let filename = attachment
                .name
                .clone()
                .unwrap_or_else(|| format!("{}.bin", blob_id));

            let content_type = attachment.content_type.clone().unwrap_or_default();
            let bytes = client.download_blob(blob_id).await?;

            let text = extract_text(bytes.as_ref(), &filename).await?;

            results.push(AttachmentContent {
                filename,
                content_type,
                size: bytes.len(),
                text,
            });
        }

        Output::success(results).print();
        return Ok(());
    }

    // Default: download to files
    let out_dir = output_dir.unwrap_or(".");
    let mut downloaded: Vec<String> = Vec::new();

    for attachment in attachments {
        let blob_id = match &attachment.blob_id {
            Some(id) => id,
            None => continue,
        };

        let filename = attachment
            .name
            .clone()
            .unwrap_or_else(|| format!("{}.bin", blob_id));

        let content_type = attachment
            .content_type
            .as_deref()
            .unwrap_or("application/octet-stream");

        let bytes = client.download_blob(blob_id).await?;

        // Resize images if --max-size specified.
        // `Bytes` derefs to `&[u8]`; only the resize path allocates a new Vec<u8>.
        let (final_data, final_filename): (std::borrow::Cow<[u8]>, String) =
            if let Some(max) = max_bytes {
                let mime = if is_image(content_type, &filename) {
                    infer_image_mime(&filename).unwrap_or(content_type)
                } else {
                    content_type
                };

                if is_image(mime, &filename) {
                    match resize_image(bytes.as_ref(), mime, max) {
                        Ok((resized, new_mime)) => {
                            // Update extension if format changed (e.g., PNG -> JPEG)
                            let new_filename = if new_mime == "image/jpeg"
                                && !filename.to_lowercase().ends_with(".jpg")
                                && !filename.to_lowercase().ends_with(".jpeg")
                            {
                                let stem = Path::new(&filename)
                                    .file_stem()
                                    .and_then(|s| s.to_str())
                                    .unwrap_or(&filename);
                                format!("{}.jpg", stem)
                            } else {
                                filename.clone()
                            };
                            (std::borrow::Cow::Owned(resized), new_filename)
                        }
                        Err(_) => (std::borrow::Cow::Borrowed(bytes.as_ref()), filename.clone()),
                    }
                } else {
                    (std::borrow::Cow::Borrowed(bytes.as_ref()), filename.clone())
                }
            } else {
                (std::borrow::Cow::Borrowed(bytes.as_ref()), filename.clone())
            };

        let path = Path::new(out_dir).join(safe_filename(&final_filename));
        std::fs::write(&path, &final_data)?;

        downloaded.push(path.to_string_lossy().to_string());
    }

    #[derive(serde::Serialize)]
    struct DownloadResponse {
        files: Vec<String>,
    }

    Output::success(DownloadResponse { files: downloaded }).print();

    Ok(())
}

#[derive(serde::Serialize)]
struct AttachmentContent {
    filename: String,
    content_type: String,
    size: usize,
    text: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::safe_filename;

    #[test]
    fn test_safe_filename_strips_parent_traversal() {
        assert_eq!(safe_filename("../../../etc/passwd"), "passwd");
    }

    #[test]
    fn test_safe_filename_strips_absolute_path() {
        assert_eq!(safe_filename("/absolute/path/file.pdf"), "file.pdf");
    }

    #[test]
    fn test_safe_filename_dotdot_falls_back_to_attachment() {
        assert_eq!(safe_filename(".."), "attachment");
    }

    #[test]
    fn test_safe_filename_empty_falls_back_to_attachment() {
        assert_eq!(safe_filename(""), "attachment");
    }

    #[test]
    fn test_safe_filename_plain_name_preserved() {
        assert_eq!(safe_filename("normal.txt"), "normal.txt");
    }
}
