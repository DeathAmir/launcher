import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import Persian from "./translations/fa";

export type LanguageType = "fa";

interface LanguageResource {
  label: string;
  type: LanguageType;
  translations: { [key: string]: string };
}

export const getLanguageMetadata = () => ({
  fa: { label: "فارسی", type: "fa" as const },
});

export const getLanguages = (): Record<string, LanguageResource> => ({
  fa: { label: "فارسی", type: "fa", translations: Persian },
});

export const loadLanguageTranslations = async (_lang?: LanguageType): Promise<{ [key: string]: string }> => Persian;

i18n.use(initReactI18next).init({
  resources: { fa: { translations: Persian } },
  lng: "fa",
  fallbackLng: "fa",
  debug: false,
  ns: ["translations"],
  defaultNS: "translations",
  keySeparator: false,
  interpolation: { escapeValue: false },
  load: "currentOnly",
  react: { useSuspense: false },
});

export const changeLanguage = async (_lang?: LanguageType): Promise<void> => {
  await i18n.changeLanguage("fa");
};

export default i18n;
