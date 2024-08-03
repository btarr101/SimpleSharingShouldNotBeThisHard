
/**
 * Used to update the counter on the `shared" page.
 * 
 * @param {Number} ms 
 * @returns Human readable milliseconds duration.
 */
function formatDuration(ms) {
    const time = {
        days: Math.floor(ms / 86400000),
        h: Math.floor(ms / 3600000) % 24,
        m: Math.floor(ms / 60000) % 60,
        s: Math.floor(ms / 1000) % 60,
    };
    return Object.entries(time)
    .filter(val => val[1] !== 0)
    .map(([key, val]) => `${val}${key}`)
    .join(' ');
};
