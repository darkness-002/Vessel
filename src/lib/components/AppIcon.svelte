<script lang="ts">
  interface Props {
    name?: string;
    url?: string;
    icon?: string | null;
    size?: number; // Tile dimension in px (default: 40)
    rounded?: string; // e.g. 'rounded-xl', 'rounded-lg'
    className?: string;
  }

  let {
    name = '',
    url = '',
    icon = null,
    size = 40,
    rounded = 'rounded-xl',
    className = '',
  }: Props = $props();

  let imgFailed = $state(false);
  let ddgFailed = $state(false);
  let googleFaviconFailed = $state(false);

  function getDomain(rawUrl: string): string {
    if (!rawUrl) return '';
    try {
      const u = rawUrl.startsWith('http') ? rawUrl : `https://${rawUrl}`;
      const parsed = new URL(u);
      return parsed.hostname.replace(/^www\./, '').toLowerCase();
    } catch {
      return rawUrl.replace(/^https?:\/\//, '').split('/')[0].toLowerCase();
    }
  }

  type BrandInfo = {
    id: string;
    bg: string;
    fg: string;
    svgType: string;
  };

  const POPULAR_APPS = ['github', 'linear', 'notion', 'slack', 'spotify'];

  function resolveBrand(nameStr: string, urlStr: string): BrandInfo | null {
    const n = (nameStr || '').toLowerCase();
    const d = getDomain(urlStr);

    if (d.includes('spotify') || n.includes('spotify')) {
      return { id: 'spotify', bg: '#1ED760', fg: '#000000', svgType: 'spotify' };
    }
    if (d.includes('github') || n.includes('github')) {
      return { id: 'github', bg: '#24292F', fg: '#FFFFFF', svgType: 'github' };
    }
    if (d.includes('linear.app') || n.includes('linear')) {
      return { id: 'linear', bg: '#5E6AD2', fg: '#FFFFFF', svgType: 'linear' };
    }
    if (d.includes('slack') || n.includes('slack')) {
      return { id: 'slack', bg: '#4A154B', fg: '#FFFFFF', svgType: 'slack' };
    }
    if (d.includes('notion') || n.includes('notion')) {
      return { id: 'notion', bg: '#000000', fg: '#FFFFFF', svgType: 'notion' };
    }
    if (d.includes('gmail') || d.includes('mail.google') || n.includes('gmail') || n.includes('mail')) {
      return { id: 'mail', bg: '#EA4335', fg: '#FFFFFF', svgType: 'mail' };
    }
    if (n.includes('note') || d.includes('note')) {
      return { id: 'notes', bg: '#F59E0B', fg: '#FFFFFF', svgType: 'notes' };
    }
    if (d.includes('discord') || n.includes('discord')) {
      return { id: 'discord', bg: '#5865F2', fg: '#FFFFFF', svgType: 'discord' };
    }
    if (d.includes('chatgpt') || d.includes('openai') || n.includes('chatgpt')) {
      return { id: 'chatgpt', bg: '#10A37F', fg: '#FFFFFF', svgType: 'chatgpt' };
    }
    if (d.includes('figma') || n.includes('figma')) {
      return { id: 'figma', bg: '#1E1E1E', fg: '#FFFFFF', svgType: 'figma' };
    }
    if (d.includes('youtube') || n.includes('youtube')) {
      return { id: 'youtube', bg: '#FF0000', fg: '#FFFFFF', svgType: 'youtube' };
    }
    if (d.includes('x.com') || d.includes('twitter') || n.includes('twitter')) {
      return { id: 'twitter', bg: '#000000', fg: '#FFFFFF', svgType: 'twitter' };
    }
    if (d.includes('reddit') || n.includes('reddit')) {
      return { id: 'reddit', bg: '#FF4500', fg: '#FFFFFF', svgType: 'reddit' };
    }
    if (d.includes('whatsapp') || n.includes('whatsapp')) {
      return { id: 'whatsapp', bg: '#25D366', fg: '#FFFFFF', svgType: 'whatsapp' };
    }
    if (d.includes('telegram') || n.includes('telegram')) {
      return { id: 'telegram', bg: '#229ED9', fg: '#FFFFFF', svgType: 'telegram' };
    }
    if (d.includes('linkedin') || n.includes('linkedin')) {
      return { id: 'linkedin', bg: '#0A66C2', fg: '#FFFFFF', svgType: 'linkedin' };
    }
    return null;
  }

  let brand = $derived(resolveBrand(name, url));
  let isPopularBrand = $derived(Boolean(brand && POPULAR_APPS.includes(brand.id)));
  let domain = $derived(getDomain(url));

  let ddgFaviconUrl = $derived(
    domain && !domain.includes('localhost') && domain.includes('.')
      ? `https://icons.duckduckgo.com/ip3/${domain}.ico`
      : null
  );

  let googleFaviconUrl = $derived(
    domain && !domain.includes('localhost') && domain.includes('.')
      ? `https://www.google.com/s2/favicons?domain=${domain}&sz=128`
      : null
  );

  let initial = $derived((name || domain || 'V').trim().charAt(0).toUpperCase());
  let svgDimension = $derived(Math.round(size * 0.55));

  // Clean SVG vector fallback is preferred for popular apps or when no custom image is available
  let useBrandVector = $derived(
    Boolean(brand && (isPopularBrand || (!icon || imgFailed)))
  );
</script>

<div
  class="{rounded} aspect-square flex items-center justify-center shrink-0 overflow-hidden select-none transition-colors shadow-xs {className}"
  style="width: {size}px; height: {size}px; min-width: {size}px; min-height: {size}px; border-radius: 10px; background-color: {useBrandVector && brand
    ? brand.bg
    : 'var(--bg-canvas)'}; border: 1px solid rgba(255, 255, 255, 0.08); overflow: hidden;"
>
  {#if useBrandVector && brand}
    <!-- Official Brand Vector SVGs with Official Background Colors and centered SVG padding -->
    {#if brand.svgType === 'spotify'}
      <svg width={svgDimension} height={svgDimension} viewBox="0 0 24 24" fill={brand.fg} aria-label="Spotify">
        <path d="M12 0C5.373 0 0 5.373 0 12s5.373 12 12 12 12-5.373 12-12S18.627 0 12 0zm5.498 17.306c-.218.358-.684.472-1.042.254-2.858-1.748-6.455-2.143-10.693-1.173-.41.094-.824-.162-.918-.572-.094-.41.162-.824.572-.918 4.636-1.06 8.607-.611 11.827 1.367.358.218.472.684.254 1.042zm1.467-3.262c-.275.447-.859.59-1.306.315-3.27-2.01-8.256-2.593-12.124-1.418-.501.152-1.033-.134-1.185-.635-.152-.501.134-1.033.635-1.185 4.417-1.34 9.907-.688 13.665 1.618.447.275.59.859.315 1.306zm.126-3.41c-3.921-2.328-10.383-2.543-14.127-1.406-.601.182-1.238-.16-1.42-.761-.182-.601.16-1.238.761-1.42 4.301-1.306 11.436-1.055 15.938 1.619.54.321.719 1.026.398 1.566-.321.54-1.026.719-1.566.398z"/>
      </svg>
    {:else if brand.svgType === 'github'}
      <svg width={svgDimension} height={svgDimension} viewBox="0 0 24 24" fill={brand.fg} aria-label="GitHub">
        <path fill-rule="evenodd" clip-rule="evenodd" d="M12 2C6.477 2 2 6.484 2 12.017c0 4.425 2.865 8.18 6.839 9.504.5.092.682-.217.682-.483 0-.237-.008-.868-.013-1.703-2.782.605-3.369-1.343-3.369-1.343-.454-1.158-1.11-1.466-1.11-1.466-.908-.62.069-.608.069-.608 1.003.07 1.53 1.032 1.53 1.032.892 1.53 2.341 1.088 2.91.832.092-.647.35-1.088.636-1.338-2.22-.253-4.555-1.113-4.555-4.951 0-1.093.39-1.988 1.029-2.688-.103-.253-.446-1.272.098-2.65 0 0 .84-.27 2.75 1.026A9.564 9.564 0 0112 6.844c.85.004 1.705.115 2.504.337 1.909-1.296 2.747-1.027 2.747-1.027.546 1.379.202 2.398.1 2.651.64.7 1.028 1.595 1.028 2.688 0 3.848-2.339 4.695-4.566 4.943.359.309.678.92.678 1.855 0 1.338-.012 2.419-.012 2.747 0 .268.18.58.688.482A10.019 10.019 0 0022 12.017C22 6.484 17.522 2 12 2z"/>
      </svg>
    {:else if brand.svgType === 'linear'}
      <svg width={svgDimension} height={svgDimension} viewBox="0 0 24 24" fill={brand.fg} aria-label="Linear">
        <path d="M3.568 7.375a8.625 8.625 0 0 1 13.057-3.807L3.75 16.443a8.59 8.59 0 0 1-.182-9.068zm1.968 10.875L18.411 5.375a8.625 8.625 0 0 1-12.875 12.875zM12 2C6.477 2 2 6.477 2 12s4.477 10 10 10 10-4.477 10-10S17.523 2 12 2z"/>
      </svg>
    {:else if brand.svgType === 'slack'}
      <svg width={svgDimension} height={svgDimension} viewBox="0 0 24 24" fill={brand.fg} aria-label="Slack">
        <path d="M5.042 15.165a2.528 2.528 0 0 1-2.52 2.523A2.528 2.528 0 0 1 0 15.165a2.527 2.527 0 0 1 2.522-2.52h2.52v2.52zM6.313 15.165a2.527 2.527 0 0 1 2.521-2.52 2.527 2.527 0 0 1 2.521 2.52v6.313A2.528 2.528 0 0 1 8.834 24a2.528 2.528 0 0 1-2.521-2.522v-6.313zM8.834 5.042a2.528 2.528 0 0 1-2.521-2.52A2.528 2.528 0 0 1 8.834 0a2.528 2.528 0 0 1 2.521 2.522v2.52H8.834zM8.834 6.313a2.528 2.528 0 0 1 2.521 2.521 2.528 2.528 0 0 1-2.521 2.521H2.522A2.528 2.528 0 0 1 0 8.834a2.528 2.528 0 0 1 2.522-2.521h6.312zM18.956 8.834a2.528 2.528 0 0 1 2.522-2.521A2.528 2.528 0 0 1 24 8.834a2.528 2.528 0 0 1-2.522 2.521h-2.522V8.834zM17.688 8.834a2.528 2.528 0 0 1-2.523 2.521 2.527 2.527 0 0 1-2.52-2.521V2.522A2.527 2.527 0 0 1 15.165 0a2.528 2.528 0 0 1 2.523 2.522v6.312zM15.165 18.956a2.528 2.528 0 0 1 2.523 2.522A2.528 2.528 0 0 1 15.165 24a2.527 2.527 0 0 1-2.52-2.522v-2.522h2.52zM15.165 17.688a2.527 2.527 0 0 1-2.52-2.523 2.526 2.526 0 0 1 2.52-2.52h6.313A2.527 2.527 0 0 1 24 15.165a2.528 2.528 0 0 1-2.522 2.523h-6.313z"/>
      </svg>
    {:else if brand.svgType === 'notion'}
      <svg width={svgDimension} height={svgDimension} viewBox="0 0 24 24" fill={brand.fg} aria-label="Notion">
        <path d="M4.459 4.208c.746.606 1.026.56 2.428.466l13.215-.793c.28 0 .047-.28-.047-.326L17.86 1.64c-.467-.373-1.214-.84-2.428-.747L2.918 1.826c-.467.047-.56.327-.374.514zm-.84 3.734v13.636c0 .84.42 1.12 1.307 1.073l14.288-.84c.887-.046 1.027-.56 1.027-1.213V7.054c0-.653-.28-.98-.84-.933l-14.894.887c-.654.046-.888.373-.888.934zm13.121 2.193c.094.42 0 .84-.42.887l-.7.14v8.121c-.42.233-.84.373-1.213.373-.654 0-.84-.233-1.354-.887l-5.32-8.354v7.934l1.587.373s0 .747-.933.747l-3.36.187s0-.42.373-.56l.84-.234V10.74l-1.307-.14s0-.56.654-.607l3.64-.233 5.46 8.354V10.6l-1.4-.187s0-.606.793-.653z"/>
      </svg>
    {:else if brand.svgType === 'mail'}
      <svg width={svgDimension} height={svgDimension} viewBox="0 0 24 24" fill={brand.fg} aria-label="Mail">
        <path d="M20 4H4c-1.1 0-2 .9-2 2v12c0 1.1.9 2 2 2h16c1.1 0 2-.9 2-2V6c0-1.1-.9-2-2-2zm0 4-8 5-8-5V6l8 5 8-5v2z"/>
      </svg>
    {:else if brand.svgType === 'notes'}
      <svg width={svgDimension} height={svgDimension} viewBox="0 0 24 24" fill="none" stroke={brand.fg} stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" aria-label="Notes">
        <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/>
        <polyline points="14 2 14 8 20 8"/>
        <line x1="16" y1="13" x2="8" y2="13"/>
        <line x1="16" y1="17" x2="8" y2="17"/>
        <line x1="10" y1="9" x2="8" y2="9"/>
      </svg>
    {:else if brand.svgType === 'discord'}
      <svg width={svgDimension} height={svgDimension} viewBox="0 0 24 24" fill={brand.fg} aria-label="Discord">
        <path d="M20.317 4.37a19.791 19.791 0 0 0-4.885-1.515.074.074 0 0 0-.079.037c-.21.375-.444.864-.608 1.25a18.27 18.27 0 0 0-5.487 0 12.64 12.64 0 0 0-.617-1.25.077.077 0 0 0-.079-.037A19.736 19.736 0 0 0 3.677 4.37a.07.07 0 0 0-.032.027C.533 9.046-.32 13.58.099 18.057a.082.082 0 0 0 .031.057 19.9 19.9 0 0 0 5.993 3.03.078.078 0 0 0 .084-.028c.462-.63.874-1.295 1.226-1.994.021-.041.001-.09-.041-.106a13.107 13.107 0 0 1-1.872-.892.077.077 0 0 1-.008-.128 10.2 10.2 0 0 0 .372-.292.074.074 0 0 1 .077-.01c3.929 1.793 8.18 1.793 12.061 0a.074.074 0 0 1 .078.01c.12.098.246.198.373.292a.077.077 0 0 1-.006.127 12.299 12.299 0 0 1-1.873.894.077.077 0 0 0-.041.107c.36.698.772 1.362 1.225 1.993a.076.076 0 0 0 .084.028 19.839 19.839 0 0 0 6.002-3.03.077.077 0 0 0 .032-.054c.5-5.177-.838-9.674-3.549-13.66a.061.061 0 0 0-.031-.028zM8.02 15.33c-1.183 0-2.157-1.085-2.157-2.419 0-1.333.956-2.419 2.157-2.419 1.21 0 2.176 1.096 2.157 2.42 0 1.333-.956 2.418-2.157 2.418zm7.975 0c-1.183 0-2.157-1.085-2.157-2.419 0-1.333.955-2.419 2.157-2.419 1.21 0 2.176 1.096 2.157 2.42 0 1.333-.946 2.418-2.157 2.418z"/>
      </svg>
    {:else if brand.svgType === 'chatgpt'}
      <svg width={svgDimension} height={svgDimension} viewBox="0 0 24 24" fill={brand.fg} aria-label="ChatGPT">
        <path d="M22.282 9.821a5.985 5.985 0 0 0-.516-4.91 6.046 6.046 0 0 0-6.51-2.9A6.065 6.065 0 0 0 4.981 4.18a5.985 5.985 0 0 0-3.998 2.9 6.046 6.046 0 0 0 .743 7.097 5.98 5.98 0 0 0 .51 4.911 6.051 6.051 0 0 0 6.515 2.9A5.985 5.985 0 0 0 13.26 24a6.056 6.056 0 0 0 5.772-4.206 5.99 5.99 0 0 0 3.997-2.9 6.056 6.056 0 0 0-.747-7.073zM13.26 22.43a4.476 4.476 0 0 1-2.876-1.04l.141-.081 4.779-2.758a.795.795 0 0 0 .392-.681v-6.737l2.02 1.168a.071.071 0 0 1 .038.052v5.583a4.504 4.504 0 0 1-4.494 4.494zM3.6 18.304a4.47 4.47 0 0 1-.535-3.014l.142.085 4.783 2.759a.771.771 0 0 0 .78 0l5.843-3.369v2.332a.08.08 0 0 1-.033.062L9.74 19.95a4.5 4.5 0 0 1-6.14-1.646zm-1.04-8.87a4.476 4.476 0 0 1 2.34-1.974v5.674a.795.795 0 0 0 .392.681l5.84 3.37-2.02 1.167a.076.076 0 0 1-.07.005L4.198 15.54a4.504 4.504 0 0 1-1.638-6.106zm16.657 3.368-5.844-3.37 2.02-1.167a.076.076 0 0 1 .071-.005l4.843 2.796a4.494 4.494 0 0 1-.682 8.118v-5.69a.795.795 0 0 0-.408-.682zm2.01-3.013l-.141-.085-4.779-2.76a.776.776 0 0 0-.78 0L9.684 10.31V7.98a.08.08 0 0 1 .033-.062l4.84-2.796a4.5 4.5 0 0 1 6.69 4.67zM10.864 12.33l-2.02-1.166 5.84-3.37 2.02 1.166-5.84 3.37zm1.136-1.93l2.87-1.656 2.87 1.656-2.87 1.656-2.87-1.656z"/>
      </svg>
    {:else if brand.svgType === 'figma'}
      <svg width={svgDimension} height={svgDimension} viewBox="0 0 24 24" fill={brand.fg} aria-label="Figma">
        <path d="M15.333 8.667a3.333 3.333 0 1 0 0-6.667h-6.666v6.667h6.666zm-6.666 0h6.666a3.333 3.333 0 1 1-3.333 3.333v-3.333zm-3.334 3.333a3.333 3.333 0 1 1 3.334-3.333v3.333H5.333zm0 3.333H8.667v3.334a3.333 3.333 0 1 1-3.334-3.334zm3.334-3.333H5.333a3.333 3.333 0 0 0 0 6.667H8.667v-6.667z"/>
      </svg>
    {:else if brand.svgType === 'youtube'}
      <svg width={svgDimension} height={svgDimension} viewBox="0 0 24 24" fill={brand.fg} aria-label="YouTube">
        <path d="M23.498 6.186a3.016 3.016 0 0 0-2.122-2.136C19.505 3.545 12 3.545 12 3.545s-7.505 0-9.377.505A3.017 3.017 0 0 0 .502 6.186C0 8.07 0 12 0 12s0 3.93.502 5.814a3.016 3.016 0 0 0 2.122 2.136c1.871.505 9.376.505 9.376.505s7.505 0 9.377-.505a3.015 3.015 0 0 0 2.122-2.136C24 15.93 24 12 24 12s0-3.93-.502-5.814zM9.545 15.568V8.432L15.818 12l-6.273 3.568z"/>
      </svg>
    {:else if brand.svgType === 'twitter'}
      <svg width={svgDimension} height={svgDimension} viewBox="0 0 24 24" fill={brand.fg} aria-label="X">
        <path d="M18.244 2.25h3.308l-7.227 8.26 8.502 11.24H16.17l-5.214-6.817L4.99 21.75H1.68l7.73-8.835L1.254 2.25H8.08l4.713 6.231zm-1.161 17.52h1.833L7.084 4.126H5.117z"/>
      </svg>
    {:else if brand.svgType === 'reddit'}
      <svg width={svgDimension} height={svgDimension} viewBox="0 0 24 24" fill={brand.fg} aria-label="Reddit">
        <path d="M12 0A12 12 0 0 0 0 12a12 12 0 0 0 12 12 12 12 0 0 0 12-12A12 12 0 0 0 12 0zm5.01 4.744c.688 0 1.25.561 1.25 1.249a1.25 1.25 0 0 1-2.498.056l-2.597-.547-.8 3.747c1.824.07 3.48.632 4.674 1.488.308-.309.73-.491 1.207-.491.968 0 1.754.786 1.754 1.754 0 .716-.435 1.333-1.01 1.614a3.111 3.111 0 0 1 .042.52c0 2.694-3.13 4.87-7.004 4.87-3.874 0-7.004-2.176-7.004-4.87 0-.183.015-.366.043-.534A1.748 1.748 0 0 1 4.028 12c0-.968.786-1.754 1.754-1.754.463 0 .898.196 1.207.49 1.207-.883 2.878-1.43 4.744-1.487l.885-4.182a.342.342 0 0 1 .14-.197.35.35 0 0 1 .238-.042l2.906.617a1.214 1.214 0 0 1 1.108-.701zM9.25 12C8.561 12 8 12.562 8 13.25c0 .687.561 1.248 1.25 1.248.687 0 1.248-.561 1.248-1.249 0-.688-.561-1.249-1.249-1.249zm5.5 0c-.687 0-1.248.561-1.248 1.25 0 .687.561 1.248 1.249 1.248.688 0 1.249-.561 1.249-1.249 0-.687-.562-1.249-1.25-1.249zm-5.466 3.99a.327.327 0 0 0-.231.094.33.33 0 0 0 0 .463c.842.842 2.484.913 2.961.913.477 0 2.105-.056 2.961-.913a.361.361 0 0 0 .029-.463.33.33 0 0 0-.464 0c-.547.533-1.684.73-2.512.73-.828 0-1.979-.197-2.512-.73a.326.326 0 0 0-.232-.095z"/>
      </svg>
    {:else if brand.svgType === 'whatsapp'}
      <svg width={svgDimension} height={svgDimension} viewBox="0 0 24 24" fill={brand.fg} aria-label="WhatsApp">
        <path d="M.057 24l1.687-6.163c-1.041-1.804-1.588-3.849-1.587-5.946.003-6.556 5.338-11.891 11.893-11.891 3.181.001 6.167 1.24 8.413 3.488 2.245 2.248 3.481 5.236 3.48 8.414-.003 6.557-5.338 11.892-11.893 11.892-1.99-.001-3.951-.5-5.688-1.448l-6.305 1.654zm6.597-3.807c1.676.995 3.276 1.591 5.392 1.592 5.448 0 9.886-4.434 9.889-9.885.002-5.462-4.415-9.89-9.881-9.892-5.452 0-9.887 4.434-9.889 9.884-.001 2.225.651 3.891 1.746 5.634l-.999 3.648 3.742-.981zm11.387-5.464c-.074-.124-.272-.198-.57-.347-.297-.149-1.758-.868-2.031-.967-.272-.099-.47-.149-.669.149-.198.297-.768.967-.941 1.165-.173.198-.347.223-.644.074-.297-.149-1.255-.462-2.39-1.475-.883-.788-1.48-1.761-1.653-2.059-.173-.297-.018-.458.13-.606.134-.133.297-.347.446-.521.151-.172.2-.296.3-.495.099-.198.05-.372-.025-.521-.075-.148-.669-1.611-.916-2.206-.242-.579-.487-.501-.669-.51l-.57-.01c-.198 0-.52.074-.792.372s-1.04 1.016-1.04 2.479 1.065 2.876 1.213 3.074c.149.198 2.095 3.2 5.076 4.487.709.306 1.263.489 1.694.626.712.226 1.36.194 1.872.118.571-.085 1.758-.719 2.006-1.413.248-.695.248-1.29.173-1.414z"/>
      </svg>
    {:else if brand.svgType === 'telegram'}
      <svg width={svgDimension} height={svgDimension} viewBox="0 0 24 24" fill={brand.fg} aria-label="Telegram">
        <path d="M11.944 0A12 12 0 0 0 0 12a12 12 0 0 0 12 12 12 12 0 0 0 12-12A12 12 0 0 0 12 0a12 12 0 0 0-.056 0zm4.962 7.224c.1-.002.321.023.465.14a.506.506 0 0 1 .171.325c.016.093.036.306.02.472-.18 1.898-.962 6.502-1.36 8.627-.168.9-.499 1.201-.82 1.23-.696.065-1.225-.46-1.9-.902-1.056-.693-1.653-1.124-2.678-1.8-1.185-.78-.417-1.21.258-1.91.177-.184 3.247-2.977 3.307-3.23.007-.032.014-.15-.056-.212s-.174-.041-.249-.024c-.106.024-1.793 1.14-5.061 3.345-.48.33-.913.49-1.302.48-.428-.008-1.252-.241-1.865-.44-.752-.245-1.349-.374-1.297-.789.027-.216.325-.437.893-.663 3.498-1.524 5.83-2.529 6.998-3.014 3.332-1.386 4.025-1.627 4.476-1.635z"/>
      </svg>
    {:else if brand.svgType === 'linkedin'}
      <svg width={svgDimension} height={svgDimension} viewBox="0 0 24 24" fill={brand.fg} aria-label="LinkedIn">
        <path d="M19 0h-14c-2.761 0-5 2.239-5 5v14c0 2.761 2.239 5 5 5h14c2.762 0 5-2.239 5-5v-14c0-2.761-2.238-5-5-5zm-11 19h-3v-11h3v11zm-1.5-12.268c-.966 0-1.75-.79-1.75-1.764s.784-1.764 1.75-1.764 1.75.79 1.75 1.764-.783 1.764-1.75 1.764zm13.5 12.268h-3v-5.604c0-3.368-4-3.113-4 0v5.604h-3v-11h3v1.765c1.396-2.586 7-2.777 7 2.476v6.759z"/>
      </svg>
    {/if}
  {:else if icon && !imgFailed}
    <!-- Dedicated container icon completely filling the badge tile with rounded clipping -->
    <img
      src={icon}
      alt={name}
      class="w-full h-full object-cover"
      onerror={() => {
        imgFailed = true;
      }}
    />
  {:else if ddgFaviconUrl && !ddgFailed}
    <!-- Dynamically loaded high-DPI DuckDuckGo icon -->
    <img
      src={ddgFaviconUrl}
      alt={name}
      class="w-full h-full object-cover"
      onerror={() => {
        ddgFailed = true;
      }}
    />
  {:else if googleFaviconUrl && !googleFaviconFailed}
    <!-- Fallback Google 128px favicon -->
    <img
      src={googleFaviconUrl}
      alt={name}
      class="w-full h-full object-cover"
      onerror={() => {
        googleFaviconFailed = true;
      }}
    />
  {:else}
    <!-- Monogram Fallback (crisp centered initial) -->
    <span class="text-sm font-bold text-[var(--text-primary)] uppercase select-none">
      {initial}
    </span>
  {/if}
</div>
