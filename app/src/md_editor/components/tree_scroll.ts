export function treeScrollTarget(scrollTop: number, rowTop: number, rowBottom: number, viewportTop: number, viewportBottom: number, blockBottom = rowBottom): number {
  const blockFits = Math.ceil(scrollTop + blockBottom - viewportBottom + 4) <= Math.floor(scrollTop + rowTop - viewportTop - 4);
  const bottom = blockFits ? blockBottom : rowBottom;
  const offset = Math.min(rowTop - viewportTop, Math.max(0, bottom - viewportBottom));
  return offset > 0 ? Math.ceil(scrollTop + offset + 4) : offset < 0 ? Math.floor(scrollTop + offset - 4) : scrollTop;
}
