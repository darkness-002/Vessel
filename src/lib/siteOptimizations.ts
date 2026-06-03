export function getSiteOptimizations(targetUrl: string) {
  const url = targetUrl.toLowerCase();

  if (url.includes('open.spotify.com') || url.includes('spotify.com')) {
    return {
      css: `
        .Root__top-container,
        .Root__nav-bar,
        .main-globalNav-searchContainer,
        .main-home-filterChipsSection {
          backdrop-filter: none !important;
        }

        [data-testid="context-menu"],
        [data-testid="yolo-highlight-snippet"],
        .encore-light-theme,
        #onetrust-banner-sdk,
        #onetrust-consent-sdk {
          z-index: 1 !important;
        }

        .main-nowPlayingBar-nowPlayingBar {
          contain: layout style paint;
        }
      `,
      js: `
        (() => {
          const dismissSpotifyNoise = () => {
            const selectors = [
              '#onetrust-accept-btn-handler',
              'button[data-testid="cookie-banner-accept-button"]',
              'button[aria-label="Close"]'
            ];
            for (const selector of selectors) {
              const button = document.querySelector(selector);
              if (button) {
                try { button.click(); } catch {}
              }
            }
          };

          dismissSpotifyNoise();
          setInterval(dismissSpotifyNoise, 3000);
        })();
      `
    };
  }

  return { css: '', js: '' };
}
