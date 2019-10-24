use crate::client::Graph;
use crate::http::{DownloadClient, GraphRequestType, GraphResponse, IntoResponse};
use crate::types::collection::Collection;
use crate::types::content::Content;
use graph_error::GraphRsError;
use graph_error::{AsRes, GraphFailure};
use graph_rs_types::entitytypes::{Notebook, OnenotePage, OnenoteSection, SectionGroup};
use handlebars::*;
use reqwest::header::{HeaderValue, CONTENT_TYPE};
use reqwest::Method;
use std::ffi::OsStr;
use std::fs::File;
use std::path::Path;

register_client!(
    OnenoteRequest,
    notebook => "onenote/notebooks",
    section => "onenote/sections",
    section_group => "onenote/sectionGroups",
    pages => "onenote/pages",
);

impl<'a> OnenoteRequest<'a> {
    get!( list_sections, Collection<SectionGroup> => "{{section}}" );
    get!( list_section_groups, Collection<SectionGroup> => "{{section_group}}" );
    get!( list_pages, Collection<SectionGroup> => "{{pages}}" );

    pub fn notebooks(&self) -> OnenoteNotebookRequest<'a> {
        OnenoteNotebookRequest::new(self.client)
    }

    pub fn sections(&self) -> OnenoteSectionRequest<'a> {
        OnenoteSectionRequest::new(self.client)
    }

    pub fn section_group(&self) -> OnenoteSectionGroupRequest<'a> {
        OnenoteSectionGroupRequest::new(self.client)
    }

    pub fn pages(&self) -> OnenotePageRequest<'a> {
        OnenotePageRequest::new(self.client)
    }

    pub fn create_page<P: AsRef<Path>>(&self, file: P) -> IntoResponse<'a, OnenotePage> {
        render_path!(self.client, "{{pages}}");

        if !file.as_ref().extension().eq(&Some(OsStr::new("html"))) {
            return IntoResponse::new_error(
                self.client,
                GraphRsError::InvalidFileExtension {
                    requires: "html".to_string(),
                    found: file
                        .as_ref()
                        .extension()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string(),
                }
                .as_failure(),
            );
        }

        let file = File::open(file.as_ref());
        if let Err(e) = file {
            IntoResponse::new_error(self.client, GraphFailure::from(e))
        } else {
            self.client
                .builder()
                .set_body(file.unwrap())
                .header(CONTENT_TYPE, HeaderValue::from_static("text/html"))
                .set_method(Method::POST);
            IntoResponse::new(self.client)
        }
    }
}

register_client!(OnenoteNotebookRequest,);

impl<'a> OnenoteNotebookRequest<'a> {
    get!( list, Collection<Notebook> => "{{notebook}}" );
    get!( | list_sections, Collection<OnenoteSection> => "{{notebook}}/{{id}}/sections" );
    get!( | get, Notebook => "{{notebook}}/{{id}}" );
    post!( [ create, Notebook => "{{notebook}}" ]);
    post!( [ | copy, OnenoteSection => "{{notebook}}/{{id}}/copyNotebook" ] );
    post!( [ | create_section, OnenoteSection => "{{notebook}}/{{id}}/sections" ] );

    pub fn recent(&self, include_personal_notebooks: bool) -> IntoResponse<'a, Notebook> {
        render_path!(
            self.client,
            format!(
                "{{{{notebook}}}}/getRecentNotebooks(includePersonalNotebooks={})",
                include_personal_notebooks
            ).as_str()
        );
        self.client.builder().set_method(Method::GET);
        IntoResponse::new(self.client)
    }
}

register_client!(OnenoteSectionRequest,);

impl<'a> OnenoteSectionRequest<'a> {
    get!( list, Collection<OnenoteSection> => "{{section}}" );
    get!( | list_pages, Collection<OnenotePage> => "{{section}}/{{id}}/pages" );
    get!( | get, OnenoteSection => "{{section}}/{{id}}" );
    post!( [ | copy_to_notebook, GraphResponse<Content> => "{{section}}/{{id}}/copyToNotebook" ] );
    post!( [ | copy_to_section_group, GraphResponse<Content> => "{{section}}/{{id}}/copyToSectionGroup" ] );

    pub fn create_page<S: AsRef<str>, P: AsRef<Path>>(
        &self,
        id: S,
        file: P,
    ) -> IntoResponse<'a, OnenotePage> {
        render_path!(
            self.client,
            "{{section}}/{{id}}/pages",
            &serde_json::json!({ "id": id.as_ref() })
        );

        if !file.as_ref().extension().eq(&Some(OsStr::new("html"))) {
            return IntoResponse::new_error(
                self.client,
                GraphRsError::InvalidFileExtension {
                    requires: "html".to_string(),
                    found: file
                        .as_ref()
                        .extension()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string(),
                }
                .as_failure(),
            );
        }

        let file = File::open(file.as_ref());
        if let Err(e) = file {
            IntoResponse::new_error(self.client, GraphFailure::from(e))
        } else {
            self.client
                .builder()
                .set_body(file.unwrap())
                .header(CONTENT_TYPE, HeaderValue::from_static("text/html"))
                .set_method(Method::POST);
            IntoResponse::new(self.client)
        }
    }
}

register_client!(OnenoteSectionGroupRequest,);

impl<'a> OnenoteSectionGroupRequest<'a> {
    get!( list, Collection<SectionGroup> => "{{section_group}}" );
    get!( | list_sections, Collection<OnenoteSection> => "{{section_group}}/{{id}}/sections" );
    get!( | get, SectionGroup => "{{section_group}}/{{id}}" );
    post!( [ | create, SectionGroup => "{{section_group}}/{{id}}/sectionGroups" ] );
    post!( [ | create_section, SectionGroup => "{{section_group}}/{{id}}/sections" ] );
}

register_client!(OnenotePageRequest,);

impl<'a> OnenotePageRequest<'a> {
    get!( list, Collection<OnenotePage> => "{{pages}}" );
    get!( | get, OnenotePage => "{{pages}}/{{id}}" );
    get!( | content, GraphResponse<Content> => "{{pages}}/{{id}}/content" );
    patch!( [ | update, OnenotePage => "{{pages}}/{{id}}/content" ] );
    post!( [ | copy_to_section, GraphResponse<Content> => "{{pages}}/{{id}}/copyToSection" ] );
    delete!( | delete, GraphResponse<Content> => "{{pages}}/{{id}}" );

    pub fn download<S: AsRef<str>, P: AsRef<Path>>(
        &'a self,
        id: S,
        directory: P,
    ) -> DownloadClient {
        render_path!(
            self.client,
            "{{pages}}/{{id}}/content",
            &serde_json::json!({ "id": id.as_ref() })
        );
        self.client
            .builder()
            .set_method(Method::GET)
            .set_download_dir(directory.as_ref())
            .set_request_type(GraphRequestType::Redirect);
        self.client.request().download(self.client.take_builder())
    }
}
