{
  const viewer = document.querySelector('#screenshot-viewer');
  const scroll = viewer.querySelector('.screenshot-scroll');
  const image = new Image();
  image.draggable = false;
  scroll.append(image);

  function resetZoom() {
    image.classList.remove('zoomed');
    image.style.removeProperty('width');
    scroll.scrollTo(0, 0);
  }

  if (typeof viewer.showModal === 'function') {
    for (const link of document.querySelectorAll('.screenshot')) {
      link.addEventListener('click', (event) => {
        if (event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return;
        event.preventDefault();
        image.src = link.href;
        image.alt = link.querySelector('img').alt;
        link.focus({ preventScroll: true });
        viewer.showModal();
        resetZoom();
      });
    }
    viewer.addEventListener('keydown', (event) => {
      if (event.metaKey || event.ctrlKey || event.altKey) return;
      if (event.key === '-') {
        event.preventDefault();
        resetZoom();
      } else if ((event.key === '+' || event.key === '=') && image.naturalWidth && !image.classList.contains('zoomed')) {
        event.preventDefault();
        image.style.width = `${image.getBoundingClientRect().width * 2}px`;
        image.classList.add('zoomed');
      }
    });
    let origin;
    let dragged = false;
    viewer.addEventListener('pointerdown', () => { dragged = false; });
    scroll.addEventListener('pointerdown', (event) => {
      if (event.pointerType !== 'mouse' || event.button !== 0 || !image.classList.contains('zoomed')) return;
      origin = { x: event.clientX, y: event.clientY, left: scroll.scrollLeft, top: scroll.scrollTop };
      scroll.setPointerCapture(event.pointerId);
    });
    scroll.addEventListener('pointermove', (event) => {
      if (!origin || !scroll.hasPointerCapture(event.pointerId)) return;
      const dx = event.clientX - origin.x, dy = event.clientY - origin.y;
      if (!dragged && Math.hypot(dx, dy) < 5) return;
      dragged = true;
      scroll.classList.add('panning');
      scroll.scrollTo(origin.left - dx, origin.top - dy);
    });
    scroll.addEventListener('lostpointercapture', () => {
      origin = undefined;
      scroll.classList.remove('panning');
    });
    viewer.addEventListener('click', () => {
      if (!dragged) viewer.close();
    });
  }
}
