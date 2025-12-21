import { createFileRoute } from '@tanstack/react-router'
import { createI18nSearchAPI } from 'fumadocs-core/search/server'
import { createTokenizer } from '@orama/tokenizers/mandarin'
import { source } from '@/lib/source'
import { i18n } from '@/lib/i18n'

const server = createI18nSearchAPI('advanced', {
  i18n,
  localeMap: {
    en: {
      language: 'english'
    },
    cn: {
      // 使用中文分词器
      components: {
        tokenizer: createTokenizer()
      },
      search: {
        threshold: 0,
        tolerance: 0
      }
    }
  },
  indexes: i18n.languages.map((lang) => ({
    language: lang,
    source: source.getPages(lang)
  }))
})

export const Route = createFileRoute('/api/search')({
  server: {
    handlers: {
      GET: async ({ request }) => server.GET(request)
    }
  }
})
