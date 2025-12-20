import { createFileRoute, redirect } from '@tanstack/react-router'

export const Route = createFileRoute('/')({
  beforeLoad: () => {
    // 重定向到默认语言文档首页
    throw redirect({
      to: '/en'
    })
  }
})
