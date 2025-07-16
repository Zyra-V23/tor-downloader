use crate::errors::{parser_error, Result};
use scraper::{Html, Selector};
use std::collections::HashSet;
use url::Url;

#[derive(Debug, Clone)]
pub struct ParsedUrl {
    pub url: String,
    pub url_type: UrlType,
    pub depth: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UrlType {
    Page,      // HTML pages
    Asset,     // Images, CSS, JS, etc.
    Document,  // PDFs, docs, etc.
    Archive,   // ZIP, TAR, etc.
    Other,     // Unknown type
}

pub struct HtmlParser {
    base_url: Url,
}

impl HtmlParser {
    pub fn new(base_url: &str) -> Result<Self> {
        let base_url = Url::parse(base_url)
            .map_err(|e| parser_error(&format!("Invalid base URL: {}", e)))?;
        
        Ok(HtmlParser { base_url })
    }

    pub fn parse_html(&self, html_content: &str, current_url: &str, depth: usize) -> Result<Vec<ParsedUrl>> {
        let document = Html::parse_document(html_content);
        let mut urls = HashSet::new();
        
        // Parse different types of links
        self.parse_links(&document, current_url, depth, &mut urls)?;
        self.parse_assets(&document, current_url, depth, &mut urls)?;
        self.parse_forms(&document, current_url, depth, &mut urls)?;
        
        Ok(urls.into_iter().collect())
    }

    fn parse_links(&self, document: &Html, current_url: &str, depth: usize, urls: &mut HashSet<ParsedUrl>) -> Result<()> {
        // Parse <a> tags
        let link_selector = Selector::parse("a[href]")
            .map_err(|e| parser_error(&format!("Failed to create link selector: {:?}", e)))?;

        for element in document.select(&link_selector) {
            if let Some(href) = element.value().attr("href") {
                if let Ok(parsed_url) = self.resolve_url(href, current_url) {
                    if self.is_valid_url(&parsed_url) {
                        urls.insert(ParsedUrl {
                            url: parsed_url.to_string(),
                            url_type: self.determine_url_type(&parsed_url),
                            depth: depth + 1,
                        });
                    }
                }
            }
        }

        // Parse <area> tags (image maps)
        let area_selector = Selector::parse("area[href]")
            .map_err(|e| parser_error(&format!("Failed to create area selector: {:?}", e)))?;

        for element in document.select(&area_selector) {
            if let Some(href) = element.value().attr("href") {
                if let Ok(parsed_url) = self.resolve_url(href, current_url) {
                    if self.is_valid_url(&parsed_url) {
                        urls.insert(ParsedUrl {
                            url: parsed_url.to_string(),
                            url_type: self.determine_url_type(&parsed_url),
                            depth: depth + 1,
                        });
                    }
                }
            }
        }

        Ok(())
    }

    fn parse_assets(&self, document: &Html, current_url: &str, depth: usize, urls: &mut HashSet<ParsedUrl>) -> Result<()> {
        // Parse <img> tags
        let img_selector = Selector::parse("img[src]")
            .map_err(|e| parser_error(&format!("Failed to create img selector: {:?}", e)))?;

        for element in document.select(&img_selector) {
            if let Some(src) = element.value().attr("src") {
                if let Ok(parsed_url) = self.resolve_url(src, current_url) {
                    if self.is_valid_url(&parsed_url) {
                        urls.insert(ParsedUrl {
                            url: parsed_url.to_string(),
                            url_type: UrlType::Asset,
                            depth: depth + 1,
                        });
                    }
                }
            }
        }

        // Parse <link> tags (CSS, icons, etc.)
        let link_selector = Selector::parse("link[href]")
            .map_err(|e| parser_error(&format!("Failed to create link selector: {:?}", e)))?;

        for element in document.select(&link_selector) {
            if let Some(href) = element.value().attr("href") {
                if let Ok(parsed_url) = self.resolve_url(href, current_url) {
                    if self.is_valid_url(&parsed_url) {
                        urls.insert(ParsedUrl {
                            url: parsed_url.to_string(),
                            url_type: UrlType::Asset,
                            depth: depth + 1,
                        });
                    }
                }
            }
        }

        // Parse <script> tags
        let script_selector = Selector::parse("script[src]")
            .map_err(|e| parser_error(&format!("Failed to create script selector: {:?}", e)))?;

        for element in document.select(&script_selector) {
            if let Some(src) = element.value().attr("src") {
                if let Ok(parsed_url) = self.resolve_url(src, current_url) {
                    if self.is_valid_url(&parsed_url) {
                        urls.insert(ParsedUrl {
                            url: parsed_url.to_string(),
                            url_type: UrlType::Asset,
                            depth: depth + 1,
                        });
                    }
                }
            }
        }

        // Parse <source> tags (media)
        let source_selector = Selector::parse("source[src]")
            .map_err(|e| parser_error(&format!("Failed to create source selector: {:?}", e)))?;

        for element in document.select(&source_selector) {
            if let Some(src) = element.value().attr("src") {
                if let Ok(parsed_url) = self.resolve_url(src, current_url) {
                    if self.is_valid_url(&parsed_url) {
                        urls.insert(ParsedUrl {
                            url: parsed_url.to_string(),
                            url_type: UrlType::Asset,
                            depth: depth + 1,
                        });
                    }
                }
            }
        }

        // Parse <video> and <audio> tags
        for tag in &["video", "audio"] {
            let media_selector = Selector::parse(&format!("{}[src]", tag))
                .map_err(|e| parser_error(&format!("Failed to create {} selector: {:?}", tag, e)))?;

            for element in document.select(&media_selector) {
                if let Some(src) = element.value().attr("src") {
                    if let Ok(parsed_url) = self.resolve_url(src, current_url) {
                        if self.is_valid_url(&parsed_url) {
                            urls.insert(ParsedUrl {
                                url: parsed_url.to_string(),
                                url_type: UrlType::Asset,
                                depth: depth + 1,
                            });
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn parse_forms(&self, document: &Html, current_url: &str, depth: usize, urls: &mut HashSet<ParsedUrl>) -> Result<()> {
        // Parse <form> tags
        let form_selector = Selector::parse("form[action]")
            .map_err(|e| parser_error(&format!("Failed to create form selector: {:?}", e)))?;

        for element in document.select(&form_selector) {
            if let Some(action) = element.value().attr("action") {
                if let Ok(parsed_url) = self.resolve_url(action, current_url) {
                    if self.is_valid_url(&parsed_url) {
                        urls.insert(ParsedUrl {
                            url: parsed_url.to_string(),
                            url_type: UrlType::Page,
                            depth: depth + 1,
                        });
                    }
                }
            }
        }

        Ok(())
    }

    fn resolve_url(&self, url: &str, current_url: &str) -> Result<Url> {
        let current_url = Url::parse(current_url)
            .map_err(|e| parser_error(&format!("Invalid current URL: {}", e)))?;
        
        let resolved = current_url.join(url)
            .map_err(|e| parser_error(&format!("Failed to resolve URL '{}' from '{}': {}", url, current_url, e)))?;
        
        Ok(resolved)
    }

    fn is_valid_url(&self, url: &Url) -> bool {
        // Check if it's the same scheme (http/https)
        if url.scheme() != self.base_url.scheme() {
            return false;
        }

        // Check if it's the same host (same .onion domain)
        if let (Some(base_host), Some(url_host)) = (self.base_url.host_str(), url.host_str()) {
            if base_host != url_host {
                return false;
            }
        } else {
            return false;
        }

        // Skip fragments (anchors)
        if url.fragment().is_some() {
            return false;
        }

        // Skip javascript: and mailto: URLs
        if url.scheme() == "javascript" || url.scheme() == "mailto" {
            return false;
        }

        true
    }

    fn determine_url_type(&self, url: &Url) -> UrlType {
        let path = url.path().to_lowercase();
        
        // Check file extension
        if let Some(extension) = path.split('.').last() {
            match extension {
                // Images
                "jpg" | "jpeg" | "png" | "gif" | "bmp" | "webp" | "svg" | "ico" | "tiff" | "tif" => UrlType::Asset,
                
                // Stylesheets and scripts
                "css" | "js" | "json" | "xml" | "xsl" | "xslt" => UrlType::Asset,
                
                // Documents
                "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "txt" | "rtf" | "odt" | "ods" | "odp" => UrlType::Document,
                
                // Archives
                "zip" | "rar" | "7z" | "tar" | "gz" | "bz2" | "xz" | "z" => UrlType::Archive,
                
                // Web pages
                "html" | "htm" | "php" | "asp" | "aspx" | "jsp" | "cfm" | "cgi" => UrlType::Page,
                
                // Media files
                "mp3" | "mp4" | "avi" | "mov" | "wmv" | "flv" | "webm" | "ogg" | "wav" | "m4a" => UrlType::Asset,
                
                _ => UrlType::Other,
            }
        } else {
            // No extension, likely a page
            UrlType::Page
        }
    }

    pub fn extract_text_content(&self, html_content: &str) -> Result<String> {
        let document = Html::parse_document(html_content);
        let text_selector = Selector::parse("body")
            .map_err(|e| parser_error(&format!("Failed to create body selector: {:?}", e)))?;

        let mut text_content = String::new();
        
        for element in document.select(&text_selector) {
            text_content.push_str(&element.text().collect::<String>());
        }

        Ok(text_content)
    }

    pub fn extract_title(&self, html_content: &str) -> Result<Option<String>> {
        let document = Html::parse_document(html_content);
        let title_selector = Selector::parse("title")
            .map_err(|e| parser_error(&format!("Failed to create title selector: {:?}", e)))?;

        for element in document.select(&title_selector) {
            let title = element.text().collect::<String>().trim().to_string();
            if !title.is_empty() {
                return Ok(Some(title));
            }
        }

        Ok(None)
    }

    pub fn extract_meta_description(&self, html_content: &str) -> Result<Option<String>> {
        let document = Html::parse_document(html_content);
        let meta_selector = Selector::parse("meta[name='description']")
            .map_err(|e| parser_error(&format!("Failed to create meta selector: {:?}", e)))?;

        for element in document.select(&meta_selector) {
            if let Some(content) = element.value().attr("content") {
                let description = content.trim().to_string();
                if !description.is_empty() {
                    return Ok(Some(description));
                }
            }
        }

        Ok(None)
    }
}

impl std::hash::Hash for ParsedUrl {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.url.hash(state);
    }
}

impl std::cmp::PartialEq for ParsedUrl {
    fn eq(&self, other: &Self) -> bool {
        self.url == other.url
    }
}

impl std::cmp::Eq for ParsedUrl {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_html() {
        let html = r#"
        <html>
        <body>
            <a href="/page1">Page 1</a>
            <a href="/page2">Page 2</a>
            <img src="/image.jpg">
        </body>
        </html>
        "#;
        
        let parser = HtmlParser::new("http://example.onion").unwrap();
        let urls = parser.parse_html(html, "http://example.onion/", 0).unwrap();
        
        assert_eq!(urls.len(), 3);
    }

    #[test]
    fn test_url_type_detection() {
        let parser = HtmlParser::new("http://example.onion").unwrap();
        
        let pdf_url = Url::parse("http://example.onion/doc.pdf").unwrap();
        assert_eq!(parser.determine_url_type(&pdf_url), UrlType::Document);
        
        let image_url = Url::parse("http://example.onion/image.jpg").unwrap();
        assert_eq!(parser.determine_url_type(&image_url), UrlType::Asset);
        
        let page_url = Url::parse("http://example.onion/page").unwrap();
        assert_eq!(parser.determine_url_type(&page_url), UrlType::Page);
    }
} 
