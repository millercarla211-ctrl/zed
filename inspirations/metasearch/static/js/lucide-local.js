(function () {
  var NS = 'http://www.w3.org/2000/svg';
  var DEFAULT_ATTRS = {
    viewBox: '0 0 24 24',
    fill: 'none',
    stroke: 'currentColor',
    'stroke-width': '2',
    'stroke-linecap': 'round',
    'stroke-linejoin': 'round'
  };

  var ICONS = {
    'alert-triangle': [['path', { d: 'M10.29 3.86 1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z' }], ['line', { x1: '12', y1: '9', x2: '12', y2: '13' }], ['line', { x1: '12', y1: '17', x2: '12.01', y2: '17' }]],
    'arrow-left': [['path', { d: 'm12 19-7-7 7-7' }], ['path', { d: 'M19 12H5' }]],
    'arrow-right': [['path', { d: 'M5 12h14' }], ['path', { d: 'm12 5 7 7-7 7' }]],
    'box': [['path', { d: 'M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z' }], ['path', { d: 'm3.3 7 8.7 5 8.7-5' }], ['path', { d: 'M12 22V12' }]],
    'calendar': [['path', { d: 'M8 2v4' }], ['path', { d: 'M16 2v4' }], ['rect', { x: '3', y: '4', width: '18', height: '18', rx: '2' }], ['path', { d: 'M3 10h18' }]],
    'check-circle-2': [['circle', { cx: '12', cy: '12', r: '10' }], ['path', { d: 'm9 12 2 2 4-4' }]],
    'clock': [['circle', { cx: '12', cy: '12', r: '10' }], ['path', { d: 'M12 6v6l4 2' }]],
    'code-2': [['path', { d: 'm18 16 4-4-4-4' }], ['path', { d: 'm6 8-4 4 4 4' }], ['path', { d: 'm14.5 4-5 16' }]],
    'cpu': [['rect', { x: '4', y: '4', width: '16', height: '16', rx: '2' }], ['rect', { x: '9', y: '9', width: '6', height: '6' }], ['path', { d: 'M9 1v3' }], ['path', { d: 'M15 1v3' }], ['path', { d: 'M9 20v3' }], ['path', { d: 'M15 20v3' }], ['path', { d: 'M20 9h3' }], ['path', { d: 'M20 14h3' }], ['path', { d: 'M1 9h3' }], ['path', { d: 'M1 14h3' }]],
    'flask-conical': [['path', { d: 'M10 2v8L4.72 18.58A2 2 0 0 0 6.4 22h11.2a2 2 0 0 0 1.69-3.42L14 10V2' }], ['path', { d: 'M8.5 2h7' }], ['path', { d: 'M7 16h10' }]],
    'folder-open': [['path', { d: 'm6 14 2-8h12a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h3l2 2h5' }]],
    'github': [['path', { d: 'M15 22v-4a4.8 4.8 0 0 0-1-3.2c3.3-.4 6.8-1.6 6.8-7.3A5.7 5.7 0 0 0 19.2 3 5.3 5.3 0 0 0 19.1 0S17.8-.4 15 1.3a13.4 13.4 0 0 0-6 0C6.2-.4 4.9 0 4.9 0A5.3 5.3 0 0 0 4.8 3 5.7 5.7 0 0 0 3.2 7.5c0 5.7 3.5 6.9 6.8 7.3a4.8 4.8 0 0 0-1 3.2v4' }], ['path', { d: 'M9 18c-4.5 2-5-2-7-2' }]],
    'globe': [['circle', { cx: '12', cy: '12', r: '10' }], ['path', { d: 'M2 12h20' }], ['path', { d: 'M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z' }]],
    'headphones': [['path', { d: 'M3 14v3a2 2 0 0 0 2 2h1v-8H5a2 2 0 0 0-2 2z' }], ['path', { d: 'M21 14v3a2 2 0 0 1-2 2h-1v-8h1a2 2 0 0 1 2 2z' }], ['path', { d: 'M4 12a8 8 0 0 1 16 0' }]],
    'image': [['rect', { x: '3', y: '3', width: '18', height: '18', rx: '2' }], ['circle', { cx: '9', cy: '9', r: '2' }], ['path', { d: 'm21 15-3.1-3.1a2 2 0 0 0-2.8 0L6 21' }]],
    'info': [['circle', { cx: '12', cy: '12', r: '10' }], ['path', { d: 'M12 16v-4' }], ['path', { d: 'M12 8h.01' }]],
    'layers': [['path', { d: 'm12 2 9 5-9 5-9-5 9-5z' }], ['path', { d: 'm3 12 9 5 9-5' }], ['path', { d: 'm3 17 9 5 9-5' }]],
    'list': [['line', { x1: '8', y1: '6', x2: '21', y2: '6' }], ['line', { x1: '8', y1: '12', x2: '21', y2: '12' }], ['line', { x1: '8', y1: '18', x2: '21', y2: '18' }], ['line', { x1: '3', y1: '6', x2: '3.01', y2: '6' }], ['line', { x1: '3', y1: '12', x2: '3.01', y2: '12' }], ['line', { x1: '3', y1: '18', x2: '3.01', y2: '18' }]],
    'music': [['path', { d: 'M9 18V5l12-2v13' }], ['circle', { cx: '6', cy: '18', r: '3' }], ['circle', { cx: '18', cy: '16', r: '3' }]],
    'newspaper': [['path', { d: 'M4 19.5A2.5 2.5 0 0 0 6.5 22H20' }], ['path', { d: 'M20 4H8a2 2 0 0 0-2 2v13' }], ['path', { d: 'M8 8h8' }], ['path', { d: 'M8 12h8' }], ['path', { d: 'M8 16h5' }]],
    'play': [['polygon', { points: '6 3 20 12 6 21 6 3' }]],
    'play-circle': [['circle', { cx: '12', cy: '12', r: '10' }], ['polygon', { points: '10 8 16 12 10 16 10 8' }]],
    'refresh-cw': [['path', { d: 'M21 2v6h-6' }], ['path', { d: 'M3 12a9 9 0 0 1 15-6.7L21 8' }], ['path', { d: 'M3 22v-6h6' }], ['path', { d: 'M21 12a9 9 0 0 1-15 6.7L3 16' }]],
    'save': [['path', { d: 'M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z' }], ['path', { d: 'M17 21v-8H7v8' }], ['path', { d: 'M7 3v5h8' }]],
    'search': [['circle', { cx: '11', cy: '11', r: '8' }], ['path', { d: 'm21 21-4.35-4.35' }]],
    'search-x': [['circle', { cx: '11', cy: '11', r: '8' }], ['path', { d: 'm21 21-4.35-4.35' }], ['path', { d: 'm8 8 6 6' }], ['path', { d: 'm14 8-6 6' }]],
    'shield': [['path', { d: 'M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z' }]],
    'shield-check': [['path', { d: 'M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z' }], ['path', { d: 'm9 12 2 2 4-4' }]],
    'sliders-horizontal': [['line', { x1: '21', y1: '4', x2: '14', y2: '4' }], ['line', { x1: '10', y1: '4', x2: '3', y2: '4' }], ['line', { x1: '21', y1: '12', x2: '12', y2: '12' }], ['line', { x1: '8', y1: '12', x2: '3', y2: '12' }], ['line', { x1: '21', y1: '20', x2: '16', y2: '20' }], ['line', { x1: '12', y1: '20', x2: '3', y2: '20' }], ['line', { x1: '14', y1: '2', x2: '14', y2: '6' }], ['line', { x1: '8', y1: '10', x2: '8', y2: '14' }], ['line', { x1: '16', y1: '18', x2: '16', y2: '22' }]],
    'terminal': [['polyline', { points: '4 17 10 11 4 5' }], ['line', { x1: '12', y1: '19', x2: '20', y2: '19' }]],
    'tv-2': [['rect', { x: '2', y: '7', width: '20', height: '15', rx: '2' }], ['polyline', { points: '17 2 12 7 7 2' }]],
    'users': [['path', { d: 'M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2' }], ['circle', { cx: '9', cy: '7', r: '4' }], ['path', { d: 'M22 21v-2a4 4 0 0 0-3-3.87' }], ['path', { d: 'M16 3.13a4 4 0 0 1 0 7.75' }]],
    'x': [['path', { d: 'M18 6 6 18' }], ['path', { d: 'm6 6 12 12' }]],
    'zap': [['polygon', { points: '13 2 3 14 12 14 11 22 21 10 12 10 13 2' }]]
  };

  function createElement(type, attrs) {
    var node = document.createElementNS(NS, type);
    Object.keys(attrs).forEach(function (key) {
      node.setAttribute(key, attrs[key]);
    });
    return node;
  }

  function buildIcon(name, sourceNode) {
    var spec = ICONS[name];
    if (!spec) return null;

    var svg = createElement('svg', DEFAULT_ATTRS);
    svg.setAttribute('class', 'lucide lucide-' + name + (sourceNode.className ? ' ' + sourceNode.className : ''));
    svg.setAttribute('aria-hidden', sourceNode.getAttribute('aria-hidden') || 'true');

    Array.prototype.forEach.call(sourceNode.attributes, function (attribute) {
      if (attribute.name === 'data-lucide' || attribute.name === 'class') return;
      svg.setAttribute(attribute.name, attribute.value);
    });

    spec.forEach(function (part) {
      svg.appendChild(createElement(part[0], part[1]));
    });
    return svg;
  }

  function replaceIcon(node) {
    var name = node.getAttribute('data-lucide');
    if (!name) return;
    var svg = buildIcon(name, node);
    if (!svg) return;
    node.replaceWith(svg);
  }

  function createIcons(options) {
    var roots = options && options.nodes && options.nodes.length ? options.nodes : [document];
    roots.forEach(function (root) {
      if (!root) return;
      if (root.nodeType === 1 && root.hasAttribute && root.hasAttribute('data-lucide')) {
        replaceIcon(root);
      }
      if (root.querySelectorAll) {
        root.querySelectorAll('[data-lucide]').forEach(replaceIcon);
      }
    });
  }

  window.lucide = { createIcons: createIcons };
  document.addEventListener('DOMContentLoaded', function () {
    createIcons();
  });
}());
