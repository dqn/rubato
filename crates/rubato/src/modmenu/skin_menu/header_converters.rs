// Conversion helpers: SkinHeaderData / LR2SkinHeaderData -> SkinHeader.

use super::super::{
    CustomCategory, CustomCategoryItem, CustomFile, CustomOffset, CustomOption, SkinHeader,
    SkinType,
};
use rubato_skin::json::json_skin_loader::{CustomItemData, SkinHeaderData};
use rubato_skin::lr2::lr2_skin_header_loader::LR2SkinHeaderData;

#[allow(dead_code)]
pub(super) fn skin_header_from_json_data(data: SkinHeaderData) -> SkinHeader {
    let mut header = SkinHeader::new();
    header.skin_type_id = data.header_type;
    header.set_path(data.path);
    header.set_name(data.name);
    if let Some(st) = SkinType::skin_type_by_id(data.skin_type) {
        header.set_skin_type(st);
    }
    let options: Vec<CustomOption> = data
        .custom_options
        .into_iter()
        .map(|co| CustomOption::new(co.name, co.option, co.names))
        .collect();
    header.options = options;
    let files: Vec<CustomFile> = data
        .custom_files
        .into_iter()
        .map(|cf| CustomFile::new(cf.name, cf.path, cf.def))
        .collect();
    header.files = files;
    let offsets: Vec<CustomOffset> = data
        .custom_offsets
        .into_iter()
        .map(|co| CustomOffset::new(co.name, co.id, co.caps))
        .collect();
    header.offsets = offsets;
    let categories: Vec<CustomCategory> = data
        .custom_categories
        .into_iter()
        .map(|cc| {
            let items: Vec<CustomCategoryItem> = cc
                .items
                .into_iter()
                .map(|item| match item {
                    CustomItemData::Option(co) => {
                        CustomCategoryItem::Option(CustomOption::new(co.name, co.option, co.names))
                    }
                    CustomItemData::File(cf) => {
                        CustomCategoryItem::File(CustomFile::new(cf.name, cf.path, cf.def))
                    }
                    CustomItemData::Offset(co) => {
                        CustomCategoryItem::Offset(CustomOffset::new(co.name, co.id, co.caps))
                    }
                })
                .collect();
            CustomCategory::new(cc.name, items)
        })
        .collect();
    header.categories = categories;
    if let Some(res) = data.source_resolution {
        header.set_source_resolution(res);
    }
    if let Some(res) = data.destination_resolution {
        header.set_destination_resolution(res);
    }
    header
}

#[allow(dead_code)]
pub(super) fn skin_header_from_lr2_data(data: LR2SkinHeaderData) -> SkinHeader {
    let mut header = SkinHeader::new();
    if let Some(path) = data.path {
        header.set_path(path);
    }
    if let Some(st) = data.skin_type {
        header.set_skin_type(st);
    }
    header.set_name(data.name);
    // Convert LR2-specific types to skin_header types
    let options: Vec<CustomOption> = data
        .custom_options
        .into_iter()
        .map(|co| CustomOption::new(co.name, co.option, co.contents))
        .collect();
    header.options = options;
    let files: Vec<CustomFile> = data
        .custom_files
        .into_iter()
        .map(|cf| CustomFile::new(cf.name, cf.path, cf.def))
        .collect();
    header.files = files;
    let offsets: Vec<CustomOffset> = data
        .custom_offsets
        .into_iter()
        .map(|co| CustomOffset::new(co.name, co.id, co.caps))
        .collect();
    header.offsets = offsets;
    header
}
