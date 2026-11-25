import { createContext, useContext, useEffect, useMemo, type ReactNode } from "react";

import { en, type MessageKey } from "./messages/en";
import { fr } from "./messages/fr";

export type Locale = "en" | "fr";

type Messages = Record<MessageKey, string>;
type Parameters = Record<string, string | number>;
export type Translate = (key: MessageKey, parameters?: Parameters) => string;

const CATALOGS: Record<Locale, Messages> = { en, fr };

/** Fills `{name}` placeholders; unknown ones stay visible to be noticed. */
function format(message: string, parameters?: Parameters): string {
  if (!parameters) return message;
  return message.replace(/\{(\w+)\}/g, (placeholder, name: string) =>
    name in parameters ? String(parameters[name]) : placeholder,
  );
}

export function createTranslator(locale: Locale): Translate {
  const messages = CATALOGS[locale];
  return (key, parameters) => format(messages[key], parameters);
}

/** Before settings load: follow the WebView language (it matches Windows). */
export function guessLocale(): Locale {
  return navigator.language.toLowerCase().startsWith("fr") ? "fr" : "en";
}

interface I18nValue {
  locale: Locale;
  t: Translate;
}

const I18nContext = createContext<I18nValue>({ locale: "en", t: createTranslator("en") });

export function I18nProvider({ locale, children }: { locale: Locale; children: ReactNode }) {
  const value = useMemo(() => ({ locale, t: createTranslator(locale) }), [locale]);

  useEffect(() => {
    document.documentElement.lang = locale;
  }, [locale]);

  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}

export function useI18n(): I18nValue {
  return useContext(I18nContext);
}
