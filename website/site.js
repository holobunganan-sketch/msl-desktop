const tabs = [...document.querySelectorAll('[role="tab"]')];

function selectCase(selected) {
  tabs.forEach(tab => {
    const active = tab === selected;
    tab.setAttribute('aria-selected', String(active));
    tab.tabIndex = active ? 0 : -1;
    const panel = document.getElementById(tab.getAttribute('aria-controls'));
    if (panel) panel.hidden = !active;
  });
}

tabs.forEach((tab, index) => {
  tab.addEventListener('click', () => selectCase(tab));
  tab.addEventListener('keydown', event => {
    let next = index;
    if (event.key === 'ArrowRight') next = (index + 1) % tabs.length;
    else if (event.key === 'ArrowLeft') next = (index + tabs.length - 1) % tabs.length;
    else if (event.key === 'Home') next = 0;
    else if (event.key === 'End') next = tabs.length - 1;
    else return;
    event.preventDefault();
    selectCase(tabs[next]);
    tabs[next].focus();
  });
});

fetch('/release/latest.json', { cache: 'no-cache' })
  .then(response => response.ok ? response.json() : Promise.reject())
  .then(release => {
    const renderedVersion = document.querySelector('meta[name="msl-release"]')?.content;
    // Keep cached manifests from showing a version that disagrees with this page's history.
    if (release.asset !== 'MSL-Desktop-Windows-x64.exe' || release.version !== renderedVersion || !/^\d+\.\d+\.\d+$/.test(release.version)) return;
    document.querySelectorAll('[data-release-version]').forEach(element => {
      element.textContent = `v${release.version}`;
    });
  })
  .catch(() => {});
