<script lang="ts">  /**
   * HelixLogo — Core brand mark extracted from BrandLockup.
   *
   * Pure SVG with inline gradients. No layout opinions; consumes
   * whatever space the caller assigns.
   */
  export let size: number = 28;
  export let label = 'Helix';
  export let monochrome: boolean = false;
</script>

<svg
  class="helix-logo"
  class:monochrome
  width={size}
  height={size}
  viewBox="0 0 512 512"
  fill="none"
  xmlns="http://www.w3.org/2000/svg"
  role="img"
  aria-label={`${label} icon`}
>
  <defs>
    <linearGradient id="helixCyan" x1="151" y1="83" x2="339" y2="210" gradientUnits="userSpaceOnUse">
      <stop offset="0" stop-color="#00E5FF" />
      <stop offset="0.55" stop-color="#128BFF" />
      <stop offset="1" stop-color="#6D5CFF" />
    </linearGradient>
    <linearGradient id="helixViolet" x1="359" y1="301" x2="174" y2="444" gradientUnits="userSpaceOnUse">
      <stop offset="0" stop-color="#D946FF" />
      <stop offset="0.55" stop-color="#8A5CFF" />
      <stop offset="1" stop-color="#6D5CFF" />
    </linearGradient>
    <linearGradient id="waveGradient" x1="158" y1="202" x2="361" y2="310" gradientUnits="userSpaceOnUse">
      <stop offset="0" stop-color="#128BFF" />
      <stop offset="0.48" stop-color="#6D5CFF" />
      <stop offset="1" stop-color="#D946FF" />
    </linearGradient>
    <linearGradient id="playGradient" x1="221" y1="215" x2="301" y2="297" gradientUnits="userSpaceOnUse">
      <stop offset="0" stop-color="#4F8DFF" />
      <stop offset="1" stop-color="#8A5CFF" />
    </linearGradient>
  </defs>

  <g class="logo-body" class:monochrome>
    <path d="M161 243C139 194 178 161 225 139L342 83V116C342 135 332 148 315 156L219 199C190 212 169 226 161 243Z" fill="url(#helixCyan)" />
    <path d="M350 264C371 315 332 348 282 370L190 411V375C190 354 202 340 223 331L306 294C329 284 344 274 350 264Z" fill="url(#helixViolet)" />

    <rect x="165" y="214" width="18" height="60" rx="9" fill="url(#waveGradient)" />
    <rect x="196" y="199" width="20" height="91" rx="10" fill="url(#waveGradient)" />
    <rect x="257" y="173" width="21" height="69" rx="10.5" fill="url(#waveGradient)" />
    <rect x="257" y="293" width="21" height="55" rx="10.5" fill="url(#waveGradient)" />
    <rect x="305" y="195" width="20" height="101" rx="10" fill="url(#waveGradient)" />
    <rect x="347" y="214" width="19" height="69" rx="9.5" fill="url(#waveGradient)" />

    <path d="M227 213C227 204.9 236 200.1 242.8 204.6L297.8 241.3C303.8 245.3 303.8 254.7 297.8 258.7L242.8 295.4C236 299.9 227 295.1 227 287V213Z" fill="url(#playGradient)" />
  </g>
</svg>

<style>
  .helix-logo {
    display: inline-block;
    vertical-align: middle;
    flex-shrink: 0;
  }

  .helix-logo.monochrome,
  .helix-logo.monochrome .logo-body,
  .helix-logo.monochrome :global(*) {
    fill: #9ca3af !important;
    opacity: 0.35 !important;
  }
</style>
