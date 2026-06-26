htmx.isSwapping = false;

/************\
| Timestamps |
\************/

/**
 * Add a timestamp inside of <time> that matches the current user's time zone and locale.
 */
function initializeTimestamp(element) {
    const dateTime = element.getAttribute('datetime');
    if (!dateTime) return;
    if (dateTime.includes('T')) {
        element.textContent = new Date(dateTime).toLocaleString(undefined, {
            year: "numeric", month: "long", day: "numeric",
            hour: 'numeric', minute: '2-digit'
        });
    } else {
        const now = new Date();
        const isoString = now.toISOString();
        const offsetMinutes = now.getTimezoneOffset();
        const sign = offsetMinutes > 0 ? '-' : '+';
        const offsetHours = String(Math.floor(Math.abs(offsetMinutes) / 60)).padStart(2, '0');
        const offsetMins = String(Math.abs(offsetMinutes) % 60).padStart(2, '0');
        const isoWithTimezone = `${isoString.replace("Z", "")}${sign}${offsetHours}:${offsetMins}`;
        const time = isoWithTimezone.split('T')[1];
        element.textContent = new Date(dateTime + 'T' + time).toLocaleString(undefined, {
            year: "numeric", month: "long", day: "numeric",
        });
    }
}

/********\
| Dialog |
\********/

function initializeDialog(dialog) {
    dialog.addEventListener('click', function(event) {
        const rect = dialog.getBoundingClientRect();
        const isInDialog = (rect.top <= event.clientY && event.clientY <= rect.top + rect.height &&
            rect.left <= event.clientX && event.clientX <= rect.left + rect.width);
        if (!isInDialog) {
            dialog.close();
        }
    });
}

function teardownDialog(dialog) {
}

/**************************\
| Component Initialization |
\**************************/

function initializeComponents(eventName, detail) {
    if (eventName === 'afterSettle') {
        htmx.isSwapping = false;
    }
    
    const target = detail?.target ?? document.body;
    target.querySelectorAll('[data-is]').forEach((element) => {
        const is = element.getAttribute('data-is');
        switch (is) {
            case 'timestamp': initializeTimestamp(element); break;
        }
    });

    target.querySelectorAll('dialog').forEach((element) => {
        initializeDialog(element);
    })
}
function teardownComponents(eventName, detail) {
    if (eventName === 'beforeSwap') {
        htmx.isSwapping = true;
    }

    const target = detail?.target ?? document.body;
    target.querySelectorAll('[data-is]').forEach((element) => {
        const is = element.getAttribute('data-is');
        // switch (is) {
        // }
    });

    target.querySelectorAll('dialog').forEach((element) => {
        teardownDialog(element);
    })
}
document.addEventListener('htmx:beforeHistorySave', () => teardownComponents('beforeHistorysave'));
document.addEventListener('htmx:historyRestore', () => initializeComponents('historyRestore'));
document.addEventListener('htmx:beforeSwap', (event) => teardownComponents('beforeSwap', event));
document.addEventListener('htmx:afterSettle', (event) => initializeComponents('afterSettle', event));
document.addEventListener('htmx:oobBeforeSwap', (event) => teardownComponents('oobBeforeSwap', event));
document.addEventListener('htmx:oobAfterSwap', (event) => initializeComponents('oobAfterSwap', event));
initializeComponents();

/***********\
| Copy Text |
\***********/

function copyLink(link) {
    navigator.clipboard.writeText(link);
}