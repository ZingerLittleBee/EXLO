import { createRootRoute, HeadContent, Outlet, Scripts, useParams } from '@tanstack/react-router'
import { TanstackProvider } from 'fumadocs-core/framework/tanstack'
import { defineI18nUI } from 'fumadocs-ui/i18n'
import { RootProvider } from 'fumadocs-ui/provider/base'
import type * as React from 'react'
import { i18n } from '@/lib/i18n'
import appCss from '@/styles/app.css?url'

const { provider } = defineI18nUI(i18n, {
  translations: {
    cn: {
      displayName: '中文',
      search: '搜索文档',
      searchNoResult: '没有找到结果',
      toc: '目录',
      tocNoHeadings: '没有标题',
      lastUpdate: '最后更新',
      chooseLanguage: '选择语言',
      nextPage: '下一页',
      previousPage: '上一页',
      chooseTheme: '选择主题',
      editOnGithub: '在 GitHub 上编辑'
    },
    en: {
      displayName: 'English'
    }
  }
})

export const Route = createRootRoute({
  head: () => ({
    meta: [
      {
        charSet: 'utf-8'
      },
      {
        name: 'viewport',
        content: 'width=device-width, initial-scale=1'
      },
      {
        title: 'EXLO Documentation'
      }
    ],
    links: [
      { rel: 'stylesheet', href: appCss },
      { rel: 'icon', href: '/favicon.ico', sizes: 'any' },
      { rel: 'icon', href: '/logo.svg', type: 'image/svg+xml' },
      { rel: 'apple-touch-icon', href: '/apple-icon.png' }
    ]
  }),
  component: RootComponent
})

function RootComponent() {
  return (
    <RootDocument>
      <Outlet />
    </RootDocument>
  )
}

function RootDocument({ children }: { children: React.ReactNode }) {
  const { lang } = useParams({ strict: false })

  return (
    <html suppressHydrationWarning>
      <head>
        <HeadContent />
      </head>
      <body className="flex min-h-screen flex-col">
        <TanstackProvider>
          <RootProvider i18n={provider(lang)}>{children}</RootProvider>
        </TanstackProvider>
        <Scripts />
      </body>
    </html>
  )
}
