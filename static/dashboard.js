// @ts-check

/**
 * @typedef {Object} Reading
 * @property {string} device_id
 * @property {string} arrived_timestamp
 * @property {string} processed_timestamp
 * @property {string} reading_type
 * @property {number} value
 */

const form = /** @type {HTMLFormElement} */ (document.getElementById("connect-form"));
const tokenInput = /** @type {HTMLInputElement} */ (form.elements.namedItem("token"));
const deviceIdInput = /** @type {HTMLInputElement} */ (form.elements.namedItem("device_id"));
const disconnectBtn = /** @type {HTMLButtonElement} */ (document.getElementById("disconnect-btn"));
const statusEl = /** @type {HTMLElement} */ (document.getElementById("status"));
const readingsBody = /** @type {HTMLElement} */ (document.getElementById("readings-body"));

/** @type {WebSocket | null} */
let socket = null;

const params = new URLSearchParams(location.search);
if (params.get("token")) tokenInput.value = params.get("token") ?? "";
if (params.get("device")) deviceIdInput.value = params.get("device") ?? "";

/**
 * @param {string} text
 * @param {boolean} connected
 */
function setStatus(text, connected) {
  statusEl.textContent = text;
  statusEl.className = connected ? "connected" : "disconnected";
  disconnectBtn.disabled = !connected;
}

/**
 * @param {string} token
 * @param {string} deviceId
 */
function connect(token, deviceId) {
  const proto = location.protocol === "https:" ? "wss" : "ws";
  const url = `${proto}://${location.host}/api/v1/devices/${deviceId}/readings/stream?token=${encodeURIComponent(token)}`;
  socket = new WebSocket(url);

  setStatus("connecting…", false);

  socket.addEventListener("open", () => setStatus("connected", true));

  socket.addEventListener("message", (event) => {
    /** @type {Reading} */
    const reading = JSON.parse(event.data);
    const row = document.createElement("tr");
    row.innerHTML = `
      <td>${reading.reading_type}</td>
      <td>${reading.value}</td>
      <td>${reading.arrived_timestamp}</td>
      <td>${reading.processed_timestamp}</td>
    `;
    readingsBody.prepend(row);
  });

  socket.addEventListener("close", () => setStatus("disconnected", false));
  socket.addEventListener("error", () => setStatus("connection error", false));
}

form.addEventListener("submit", (event) => {
  event.preventDefault();
  if (socket) socket.close();
  connect(tokenInput.value.trim(), deviceIdInput.value.trim());
});

disconnectBtn.addEventListener("click", () => {
  if (socket) socket.close();
});
