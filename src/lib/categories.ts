import { CategoryKey } from "./api/types";

export const categoryLabelMap: Record<CategoryKey, string> = {
  screenshot_document: "문서/스크린샷",
  people: "인물",
  food_cafe: "음식/카페",
  nature_landscape: "자연/풍경",
  city_street_travel: "도시/여행",
  pets_animals: "반려동물/동물",
  products_objects: "제품/사물",
  other: "기타",
};

export const categoryOrder: CategoryKey[] = [
  "screenshot_document",
  "people",
  "food_cafe",
  "nature_landscape",
  "city_street_travel",
  "pets_animals",
  "products_objects",
  "other",
];
