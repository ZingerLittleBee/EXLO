import { createRootRouteWithContext, HeadContent, Outlet, redirect, Scripts } from '@tanstack/react-router'
import { TanStackRouterDevtools } from '@tanstack/react-router-devtools'
import { Toaster } from '@/components/ui/sonner'
import { isFirstRun } from '@/functions/first-run'
import Header from '../components/header'
import appCss from '../index.css?url'

export interface RouterAppContext {
  isFirstRun?: boolean
}

export const Route = createRootRouteWithContext<RouterAppContext>()({
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
        title: 'Tunnl'
      }
    ],
    links: [
      {
        rel: 'stylesheet',
        href: appCss
      }
    ]
  }),
  beforeLoad: async ({ location }) => {
    // Check for first-run scenario
    const { isFirstRun: firstRun } = await isFirstRun()

    // If no users exist and not already on /setup, redirect to setup
    if (firstRun && location.pathname !== '/setup') {
      throw redirect({
        to: '/setup'
      })
    }

    return { isFirstRun: firstRun }
  },
  component: RootDocument
})

function RootDocument() {
  return (
    <html lang="en" className="dark">
      <head>
        <HeadContent />
      </head>
      <body>
        <div className="grid h-svh grid-rows-[auto_1fr]">
          <Header />
          <Outlet />
        </div>
        <Toaster richColors />
        <TanStackRouterDevtools position="bottom-left" />
        <Scripts />
      </body>
    </html>
  )
}
