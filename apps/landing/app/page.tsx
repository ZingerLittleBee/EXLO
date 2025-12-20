import { ArchitectureSection } from '@/components/architecture-section'
import { CommunitySection } from '@/components/community-section'
import { ContributionSection } from '@/components/contribution-section'
import { FeaturesSection } from '@/components/features-section'
import { Footer } from '@/components/footer'
import { HeroSection } from '@/components/hero-section'
import { InstallationSection } from '@/components/installation-section'
import { Navbar } from '@/components/navbar'
import { ScanlineOverlay } from '@/components/scanline-overlay'

export default function Home() {
  return (
    <main className="relative min-h-screen bg-background">
      <ScanlineOverlay />
      <Navbar />
      <HeroSection />
      <FeaturesSection />
      <ArchitectureSection />
      <InstallationSection />
      <ContributionSection />
      <CommunitySection />
      <Footer />
    </main>
  )
}
