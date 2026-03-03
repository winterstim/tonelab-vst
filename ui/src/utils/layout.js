

export const KNOB_SIZE = 106;
export const KNOB_GAP = 10;
export const COL_WIDTH = 110;
export const COL_GAP = 2;
export const SIDE_PADDING = 10;
export const PADDING_V = 30;

export const BASE_BODY_WIDTH = 200;
export const BASE_BODY_HEIGHT = 340;

const rowMajorPlacements = (columns, rows, count) => {
  const placements = [];
  let index = 0;

  for (let row = 1; row <= rows; row += 1) {
    for (let col = 1; col <= columns; col += 1) {
      if (index >= count) break;
      placements.push({ row, col });
      index += 1;
    }
  }

  return placements;
};

export const getKnobLayout = (paramCount) => {
  if (!paramCount || paramCount <= 0) {
    return { columns: 1, rows: 1, placements: [] };
  }


  if (paramCount === 1) {
    return { columns: 1, rows: 1, placements: [{ row: 1, col: 1 }] };
  }

  if (paramCount === 2) {
    return {
      columns: 2,
      rows: 1,
      placements: [
        { row: 1, col: 1 },
        { row: 1, col: 2 }
      ]
    };
  }

  if (paramCount === 3) {
    return {
      columns: 2,
      rows: 2,
      placements: [
        { row: 1, col: 1 },
        { row: 1, col: 2 },
        { row: 2, col: 1, colSpan: 2 }
      ]
    };
  }

  if (paramCount === 4) {
    return {
      columns: 2,
      rows: 2,
      placements: rowMajorPlacements(2, 2, 4)
    };
  }

  if (paramCount === 5) {
    return {
      columns: 3,
      rows: 2,
      placements: [
        { row: 1, col: 1 },
        { row: 1, col: 3 },
        { row: 1, col: 2, rowSpan: 2 },
        { row: 2, col: 1 },
        { row: 2, col: 3 }
      ]
    };
  }

  if (paramCount === 6) {
    return {
      columns: 3,
      rows: 2,
      placements: rowMajorPlacements(3, 2, 6)
    };
  }

  const columns = Math.ceil(paramCount / 2);
  const rows = 2;

  return {
    columns,
    rows,
    placements: rowMajorPlacements(columns, rows, paramCount)
  };
};

export const getCardMetrics = (paramCount) => {
  const { columns, rows, placements } = getKnobLayout(paramCount);

  const contentWidth = (columns * COL_WIDTH) + ((columns - 1) * COL_GAP);
  const bodyWidth = Math.max(BASE_BODY_WIDTH, contentWidth + SIDE_PADDING);

  const contentHeight = (rows * KNOB_SIZE) + ((rows - 1) * KNOB_GAP) + (PADDING_V * 2);
  const bodyHeight = Math.max(BASE_BODY_HEIGHT, contentHeight);

  const svgWidth = bodyWidth + 100;
  const svgHeight = bodyHeight + 8;

  return {
    columns,
    rows,
    placements,
    bodyWidth,
    bodyHeight,
    svgWidth,
    svgHeight
  };
};

export const getPortOffsets = (bodyWidth) => {
  return {
    inputX: 16.5,
    outputX: bodyWidth + 83.5,
    portY: 170.5
  };
};
