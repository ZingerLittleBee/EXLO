import type { LucideIcon } from 'lucide-react'
import type { ReactNode } from 'react'

interface EmptyStateProps {
  icon?: LucideIcon
  title: string
  description?: string | ReactNode
  children?: ReactNode
}

export function EmptyState({ icon: Icon, title, description, children }: EmptyStateProps) {
  return (
    <div className="py-12 text-center text-muted-foreground">
      {Icon && <Icon className="mx-auto mb-4 h-12 w-12 text-muted-foreground/50" />}
      <p className="font-medium text-lg">{title}</p>
      {description && <p className="mt-1 text-sm">{description}</p>}
      {children}
    </div>
  )
}
