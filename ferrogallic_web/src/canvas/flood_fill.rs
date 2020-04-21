// http://www.adammil.net/blog/v126_A_More_Efficient_Flood_Fill.html

/* todo
export function flood_fill(ctx, x, y, toColor) {
    const imageData = ctx.getImageData(0, 0, ctx.canvas.width, ctx.canvas.height);

    const array = new Uint32Array(imageData.data.buffer);
    const width = imageData.width;
    const height = imageData.height;

    const fromColor = array[y * width + x];

    runFloodFill(array, x, y, width, height, fromColor, toColor);

    ctx.putImageData(imageData, 0, 0);
}

function runFloodFill(array, x, y, width, height, fromColor, toColor) {
    while(true)	{
        let ox = x, oy = y;
        while(y !== 0 && array[(y-1) * width + (x)] === fromColor) y--;
        while(x !== 0 && array[(y) * width + (x-1)] === fromColor) x--;
        if(x === ox && y === oy) break;
    }
    runFloodFillCore(array, x, y, width, height, fromColor, toColor);
}

function runFloodFillCore(array, x, y, width, height, fromColor, toColor) {
    let lastRowLength = 0;
    do {
        let rowLength = 0, sx = x;
        if (lastRowLength !== 0 && array[(y) * width + (x)] === toColor) {
            do {
                if (--lastRowLength === 0) return;
            } while(array[(y) * width + (++x)] === toColor);
            sx = x;
        } else {
            for(; x !== 0 && array[(y) * width + (x-1)] === fromColor; rowLength++, lastRowLength++) {
                array[(y) * width + (--x)] = toColor;
                if(y !== 0 && array[(y-1) * width + (x)] === fromColor) runFloodFill(array, x, y-1, width, height, fromColor, toColor);
            }
        }

        for(; sx < width && array[(y) * width + (sx)] === fromColor; rowLength++, sx++) {
            array[(y) * width + (sx)] = toColor;
        }

        if(rowLength < lastRowLength) {
            for(let end=x+lastRowLength; ++sx < end; ) {
                if(array[(y) * width + (sx)] === fromColor) runFloodFillCore(array, sx, y, width, height, fromColor, toColor);
            }
        } else if(rowLength > lastRowLength && y !== 0) {
            for(let ux=x+lastRowLength; ++ux<sx; ) {
                if(array[(y-1) * width + (ux)] === fromColor) runFloodFill(array, ux, y-1, width, height, fromColor, toColor);
            }
        }
        lastRowLength = rowLength;
    } while(lastRowLength !== 0 && ++y < height);
}
*/
