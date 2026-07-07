function renderGraph(nodesData, edgesData) {
  const container = document.getElementById('graph-container');
  const width = container.clientWidth || 900;
  const height = 560;

  const svg = d3.select('#graph-svg')
    .attr('viewBox', [0, 0, width, height]);

  const g = svg.append('g');

  // Zoom
  const zoom = d3.zoom()
    .scaleExtent([0.05, 10])
    .on('zoom', (event) => g.attr('transform', event.transform));

  svg.call(zoom);

  // Controls
  document.getElementById('btn-zoom-in').onclick = () =>
    svg.transition().call(zoom.scaleBy, 1.4);
  document.getElementById('btn-zoom-out').onclick = () =>
    svg.transition().call(zoom.scaleBy, 0.7);
  document.getElementById('btn-reset').onclick = () =>
    svg.transition().call(zoom.transform, d3.zoomIdentity);

  // Arrow marker
  const defs = svg.append('defs');
  defs.append('marker')
    .attr('id', 'arrow')
    .attr('viewBox', '0 -5 10 10')
    .attr('refX', 22)
    .attr('refY', 0)
    .attr('markerWidth', 5)
    .attr('markerHeight', 5)
    .attr('orient', 'auto')
    .append('path')
    .attr('fill', '#aaaaaa')
    .attr('d', 'M0,-5L10,0L0,5');

  defs.append('marker')
    .attr('id', 'arrow-highlight')
    .attr('viewBox', '0 -5 10 10')
    .attr('refX', 22)
    .attr('refY', 0)
    .attr('markerWidth', 5)
    .attr('markerHeight', 5)
    .attr('orient', 'auto')
    .append('path')
    .attr('fill', '#e5c07b')
    .attr('d', 'M0,-5L10,0L0,5');

  // Compute in-degree for node sizing
  const inDegree = {};
  nodesData.forEach(n => inDegree[n.id] = 0);
  edgesData.forEach(e => {
    const tid = typeof e.target === 'object' ? e.target.id : e.target;
    if (inDegree[tid] !== undefined) inDegree[tid]++;
  });

  // Node radius based on in-degree
  const radiusScale = (id) => {
    const deg = inDegree[id] || 0;
    return Math.max(7, Math.min(20, 7 + deg * 1.5));
  };

  // Force simulation
  const simulation = d3.forceSimulation(nodesData)
    .force('link', d3.forceLink(edgesData)
      .id(d => d.id)
      .distance(d => {
        const tid = typeof d.target === 'object' ? d.target.id : d.target;
        return 60 + (inDegree[tid] || 0) * 5;
      }))
    .force('charge', d3.forceManyBody().strength(-250))
    .force('center', d3.forceCenter(width / 2, height / 2))
    .force('collision', d3.forceCollide(d => radiusScale(d.id) + 5));

  // Curved edges
  const link = g.append('g')
    .selectAll('path')
    .data(edgesData)
    .join('path')
    .attr('fill', 'none')
    .attr('stroke', '#cccccc')
    .attr('stroke-width', 1.2)
    .attr('marker-end', 'url(#arrow)')
    .attr('opacity', 0.7);

  // Nodes
  const node = g.append('g')
    .selectAll('circle')
    .data(nodesData)
    .join('circle')
    .attr('r', d => radiusScale(d.id))
    .attr('fill', (d, i) => {
      if (i === 0) return '#e06c75';
      if ((inDegree[d.id] || 0) > 3) return '#c678dd';
      return '#61afef';
    })
   .attr('stroke', '#ffffff') 
    .attr('stroke-width', 2)
    .style('cursor', 'pointer')
    .call(d3.drag()
      .on('start', (event, d) => {
        if (!event.active) simulation.alphaTarget(0.3).restart();
        d.fx = d.x; d.fy = d.y;
      })
      .on('drag', (event, d) => { d.fx = event.x; d.fy = event.y; })
      .on('end', (event, d) => {
        if (!event.active) simulation.alphaTarget(0);
        d.fx = null; d.fy = null;
      }));

  // Tooltip
  const tooltip = document.getElementById('tooltip');

  node
    .on('mouseover', (event, d) => {
      // Highlight neighbors
      const neighborIds = new Set();
      edgesData.forEach(e => {
        const sid = typeof e.source === 'object' ? e.source.id : e.source;
        const tid = typeof e.target === 'object' ? e.target.id : e.target;
        if (sid === d.id) neighborIds.add(tid);
        if (tid === d.id) neighborIds.add(sid);
      });

      node.attr('opacity', n =>
        n.id === d.id || neighborIds.has(n.id) ? 1 : 0.2);
      link
        .attr('stroke', e => {
          const sid = typeof e.source === 'object' ? e.source.id : e.source;
          const tid = typeof e.target === 'object' ? e.target.id : e.target;
          return sid === d.id || tid === d.id ? '#e5c07b' : '#2a2d3e';
        })
        .attr('marker-end', e => {
          const sid = typeof e.source === 'object' ? e.source.id : e.source;
          return sid === d.id ? 'url(#arrow-highlight)' : 'url(#arrow)';
        })
        .attr('opacity', e => {
          const sid = typeof e.source === 'object' ? e.source.id : e.source;
          const tid = typeof e.target === 'object' ? e.target.id : e.target;
          return sid === d.id || tid === d.id ? 1 : 0.1;
        });

      tooltip.style.opacity = '1';
      tooltip.innerHTML = `
        <strong style="color:#61afef">${d.label}</strong><br>
        <span style="color:#888;font-size:11px">${d.url}</span><br>
        <span style="color:#98c379">↑ ${inDegree[d.id] || 0} inbound links</span>
        ${d.pagerank ? `<br><span style="color:#c678dd">PageRank: ${d.pagerank}</span>` : ''}
      `;
    })
    .on('mousemove', (event) => {
      tooltip.style.left = (event.clientX + 14) + 'px';
      tooltip.style.top  = (event.clientY + 14) + 'px';
    })
    .on('mouseout', () => {
      node.attr('opacity', 1);
      link
        .attr('stroke', '#2a2d3e')
        .attr('marker-end', 'url(#arrow)')
        .attr('opacity', 0.7);
      tooltip.style.opacity = '0';
    });


  // Tick — curved paths
  simulation.on('tick', () => {
    link.attr('d', d => {
      const sx = d.source.x, sy = d.source.y;
      const tx = d.target.x, ty = d.target.y;
      const dx = tx - sx, dy = ty - sy;
      const dr = Math.sqrt(dx * dx + dy * dy) * 1.5;
      return `M${sx},${sy}A${dr},${dr} 0 0,1 ${tx},${ty}`;
    });

    node
      .attr('cx', d => d.x)
      .attr('cy', d => d.y);
  });
}