use crate::core::model::CategoryKey;

pub fn prompts_for(category: CategoryKey) -> &'static [&'static str] {
    match category {
        CategoryKey::ScreenshotDocument => &[
            "a screenshot of a document",
            "a screenshot with text and UI",
            "a photographed document or paper",
        ],
        CategoryKey::People => &[
            "a photo of people",
            "a portrait of a person",
            "people in a social scene",
        ],
        CategoryKey::FoodCafe => &[
            "a photo of food",
            "a cafe or restaurant scene",
            "a drink or dessert on a table",
        ],
        CategoryKey::NatureLandscape => &[
            "a nature landscape photo",
            "mountains, forest, ocean, or sky",
            "a scenic outdoor view",
        ],
        CategoryKey::CityStreetTravel => &[
            "a city street photo",
            "a travel landmark or tourist place",
            "buildings and urban scenery",
        ],
        CategoryKey::PetsAnimals => &[
            "a photo of an animal",
            "a pet dog or cat",
            "wildlife or animals outdoors",
        ],
        CategoryKey::ProductsObjects => &[
            "a photo of an object or product",
            "an item on a table",
            "a close-up of a thing",
        ],
        CategoryKey::Other => &[
            "a miscellaneous photo",
            "an abstract or unclear scene",
            "something else",
        ],
    }
}

pub fn all_category_prompts() -> Vec<(CategoryKey, &'static [&'static str])> {
    crate::core::model::CATEGORY_KEYS
        .iter()
        .copied()
        .map(|k| (k, prompts_for(k)))
        .collect()
}

pub fn value_keep_prompts() -> &'static [&'static str] {
    &[
        "a valuable personal photo worth keeping",
        "a meaningful photo to keep in a personal album",
        "a high quality photo worth saving",
        "an important screenshot to keep",
    ]
}

pub fn value_drop_prompts() -> &'static [&'static str] {
    &[
        "a low quality photo not worth keeping",
        "a blurry or accidental photo",
        "a duplicate or unimportant screenshot",
        "a meaningless image to delete",
    ]
}
