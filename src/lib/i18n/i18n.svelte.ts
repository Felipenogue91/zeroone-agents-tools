import { en } from './locales/en';
import { ptBR } from './locales/pt-BR';
import { zhTW } from './locales/zh-TW';
import { zhCN } from './locales/zh-CN';
import type { Locale, LocaleInfo, TranslationKey, Translations } from './types';

const locales: Record<Locale, Translations> = {
	en,
	'pt-BR': ptBR,
	'zh-TW': zhTW,
	'zh-CN': zhCN
};

const localeInfos: LocaleInfo[] = [
	{ code: 'en', label: 'EN' },
	{ code: 'pt-BR', label: 'PT' },
	{ code: 'zh-CN', label: '简' },
	{ code: 'zh-TW', label: '繁' }
];

/**
 * pt-BR is the default. The Linux webview (WebKitGTK) reports `en-US` no
 * matter the system language, so English is only used when the user picks it
 * (stored in localStorage). Chinese system languages keep their locale.
 */
export function detectLocale(languages?: readonly string[]): Locale {
	const langs = languages ?? (typeof navigator === 'undefined' ? [] : (navigator.languages ?? [navigator.language]));
	for (const lang of langs) {
		const l = lang.toLowerCase();
		if (l === 'zh-tw' || l === 'zh-hk' || l.startsWith('zh-hant')) return 'zh-TW';
		if (l.startsWith('zh')) return 'zh-CN';
	}
	return 'pt-BR';
}

function getInitialLocale(): Locale {
	try {
		const stored = localStorage.getItem('locale');
		if (stored && stored in locales) {
			return stored as Locale;
		}
	} catch {
		// localStorage unavailable
	}
	return detectLocale();
}

class I18nState {
	locale = $state<Locale>(getInitialLocale());

	constructor() {
		if (typeof document !== 'undefined') {
			document.documentElement.lang = this.locale;
		}
	}

	t(key: TranslationKey, params?: Record<string, string | number>): string {
		let value = locales[this.locale]?.[key] ?? locales.en[key] ?? key;
		if (params) {
			for (const [k, v] of Object.entries(params)) {
				value = value.replaceAll(`{${k}}`, String(v));
			}
		}
		return value;
	}

	setLocale(locale: Locale) {
		this.locale = locale;
		try {
			localStorage.setItem('locale', locale);
		} catch {
			// localStorage unavailable
		}
		if (typeof document !== 'undefined') {
			document.documentElement.lang = locale;
		}
	}

	get availableLocales(): LocaleInfo[] {
		return localeInfos;
	}

	get currentLabel(): string {
		return localeInfos.find((l) => l.code === this.locale)?.label ?? 'EN';
	}

	get nextLocale(): Locale {
		const idx = localeInfos.findIndex((l) => l.code === this.locale);
		return localeInfos[(idx + 1) % localeInfos.length].code;
	}
}

export const i18n = new I18nState();
