import { en } from './locales/en';
import { ptBR } from './locales/pt-BR';
import { zhTW } from './locales/zh-TW';
import { zhCN } from './locales/zh-CN';
import type { Locale, TranslationKey, Translations } from './types';

const locales: Record<Locale, Translations> = {
	en,
	'pt-BR': ptBR,
	'zh-TW': zhTW,
	'zh-CN': zhCN
};

/**
 * The app ships in pt-BR only (no language switch in the UI). Dev and test
 * builds may still force another locale through localStorage, which is how the
 * component and e2e suites keep asserting the English copy.
 */
function getInitialLocale(): Locale {
	if (import.meta.env.DEV) {
		try {
			const stored = localStorage.getItem('locale');
			if (stored && stored in locales) {
				return stored as Locale;
			}
		} catch {
			// localStorage unavailable
		}
	}
	return 'pt-BR';
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
}

export const i18n = new I18nState();
