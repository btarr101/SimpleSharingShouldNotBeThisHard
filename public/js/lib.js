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
    .filter((val) => val[1] !== 0)
    .map(([key, val]) => `${val}${key}`)
    .join(" ");
}

/**
 * Removes the file from the form and replaces it with just the file name
 * for generating a presigned url.
 *
 * @param {Event} event
 */
function configMultipartRequest(event) {
  /** @type {FormData} */
  const form = event.detail.parameters;

  /** @type {File} */
  const file = form.get("File");
  form.delete("File");
  form.append("Filename", file.name);

  console.log("multipart");
}

/**
 * Adds the file content as a binary octet stream to the xhr request.
 *
 * @param {Event} event
 */
function beforePresignedRequest(event) {
  /** @type {XMLHttpRequest} */
  const xhr = event.detail.xhr;
  xhr.setRequestHeader("Content-Type", "application/octet-stream");

  /** @type {File} */
  const file = event.detail.requestConfig.parameters.get("File");

  const originalSend = xhr.send;
  xhr.send = () => originalSend.call(xhr, file);
}

/**
 * Used to configure the parts parameter when the file should be split.
 *
 * @param {Event} event
 */
function configPartRequest(event) {
  /** @type {FormData} */
  const form = event.detail.parameters;
  /** @type {File} */
  const file = form.get("File");
  /** @type {number} */
  const part = form.get("Part");

  const parts = Math.ceil(file.size / 5000000);
  const partSize = Math.ceil(file.size / parts);

  let thisPartSize = partSize;
  if (part == parts - 1) {
    thisPartSize = file.size - partSize * (parts - 1);
  }

  const partStartOffset = partSize * part;
  const partOfFile = file.slice(
    partStartOffset,
    partStartOffset + thisPartSize,
    file.type
  );
  form.set("File", partOfFile);

  console.log(Array.from(event.detail.parameters.entries()));
}

/**
 *
 * @param {Node} nodeId
 * @param {string} newId
 * @param {boolean} hidden
 */
function cloneNode(nodeId, newId, hidden = true) {
  const node = document.getElementById(nodeId);
  const newNode = node.cloneNode(false);
  newNode.id = newId;
  newNode.hidden = hidden;
  return newNode;
}
