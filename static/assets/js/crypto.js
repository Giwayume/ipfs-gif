/****************\
| Upload Signing |
\****************/

function openDB() {
    return new Promise((resolve, reject) => {
        const req = indexedDB.open('ipfsgifs', 1);
        req.onupgradeneeded = () => {
            req.result.createObjectStore('keystore');
        };
        req.onsuccess = () => resolve(req.result);
        req.onerror = () => reject(req.error);
    });
}

async function idbGet(storeName, keyId) {
    const db = await openDB();
    return new Promise((resolve, reject) => {
        const tx = db.transaction(storeName, 'readonly');
        const req = tx.objectStore(storeName).get(keyId);
        req.onsuccess = () => resolve(req.result);
        req.onerror = () => reject(req.error);
    });
}

async function idbPut(storeName, keyId, value) {
    const db = await openDB();
    return new Promise((resolve, reject) => {
        const tx = db.transaction(storeName, 'readwrite');
        const req = tx.objectStore(storeName).put(value, keyId);
        req.onsuccess = () => resolve();
        req.onerror = () => reject(req.error);
    });
}

async function getOrCreateSigningKey() {
    let signingKey = await idbGet('keystore', 'uploadSigningKeyV1');
    if (signingKey?.publicKey) return signingKey;

    signingKey = await crypto.subtle.generateKey(
        { name: 'Ed25519' },
        false,
        ['sign', 'verify']
    );
    await idbPut('keystore', 'uploadSigningKeyV1', signingKey);
    return signingKey;
}

async function getPublicSigningKey() {
    const signingKey = await getOrCreateSigningKey();
    const publicKeySpki = await crypto.subtle.exportKey('spki', signingKey.publicKey);
    return btoa(String.fromCharCode(...new Uint8Array(publicKeySpki)));
}

async function signMessage(message) {
    const signingKey = await getOrCreateSigningKey();
    const data = new TextEncoder().encode(message);

    const signatureBuffer = await crypto.subtle.sign(
        { name: 'Ed25519' },
        signingKey.privateKey,
        data,
    );

    return btoa(String.fromCharCode(...new Uint8Array(signatureBuffer)));
}
