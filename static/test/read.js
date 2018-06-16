(() => {
    'use strict';

    class Ranges {
        constructor(str) {
            // e.g. -10,15,20-30,500-,l50
            const intervals = []; // [[0,9], [14,14], [19,29], [499,Infinity]]
            this.fromLast = 0; // 50

            outer:
            for (let s of str.split(',')) {
                s = s.trim();
                if (! /^(?:l\d+|\d*(?:-\d*)?)$/.test(s)) continue;

                if ('l' == s[0]) {
                    const n = +s.slice(1);
                    if (n > this.fromLast) this.fromLast = n;
                    break;
                }

                const [dBegin, dEnd] = s.split('-');
                let begin, end;

                const n = +dBegin;
                if (dEnd + 1) {
                    begin = n && n-1;
                    if (dEnd) {
                        end = +dEnd - 1;
                        if (end < begin) continue;
                        end;
                    } else { // dEnd == ''
                        end = 1/0;
                    }
                } else { // dEnd == undefined
                    if (! n) continue;
                    begin = end = n-1;
                }

                // deduplication
                for (let i=0; i<intervals.length; i++) {
                    const r = intervals[i];
                    if (! r) continue;

                    const [x, y] = r;
                    if (x <= begin && end <= y) {
                        //  [x (begin end) y]
                        continue outer;
                    } else if (begin-1 <= y && y <= end) {
                        // [x y](begin end) or [x (begin y] end)
                        //    ^^^^^^^^ adjoining
                        begin = x;
                    } else if (begin <= x && x <= end+1) {
                        // (begin end)[x y] or (begin [x end) y]
                        //        ^^^^^^ adjoining
                        end = y;
                    } else if (x < begin || end < y) { // not (begin [x y] end)
                        // [x y]...(begin end) or (begin end)...[x y]
                        continue;
                    } // else // (begin [x y] end)

                    delete intervals[i];
                }

                intervals.push([begin, end]);
            }

            this.intervals = intervals
                .filter(_ => true) // remove empty slots
                .sort(([a],[b]) => a-b);
        }

        forEachIndexTo(len, callback) {
            let [begin, end] = this.intervals[0] || [0, 1/0];
            let inRange;

            for (let i=j=0; i < len; i++) {
                if (i == end+1) {
                    [begin, end] = this.intervals[++j] || [];
                    inRange = false;
                } else if (i == begin) {
                    inRange = true;
                }
                callback(i, inRange || len - this.fromLast <= i);
            }
        }
    }

    const PATHNAME_REGEXP = /^\/test\/read\.cgi\/([A-Za-z\d]+)\/(\d+)(?:\/([^/]*))?/;
    // Terminology:                        board ^~~~~~~~~~~~~  ~~~~^ key
    // thread_id := board + key

    function addEventListenerToAnchor(a, board, key, range) {
        a.addEventListener('click', e => {
            e.preventDefault();
            go(a.href, board, key, range);
        });
    }

    // name<>mail<>datetime<> body <>title\n
    const NAME = 0;
    const MAIL = 1;
    const DATETIME = 2;
    const BODY = 3;
    const TITLE = 4;

    function createArticle(idx, fields) {
        const ret = document.createElement('article');
        ret.id = ret.dataset.id = ++idx;
        ret.className = 'post';

        const datetime = fields[DATETIME].replace(/(^| )(ID:[A-Za-z\d+./?]+)($| )/, (_, p1, p2, p3) => {
            ret.dataset.userid = p2;
            return `${$1}<span class="uid">${p2}</span>${p3}`;
        }).replace(
            /(\d{4})\/(\d{2})\/(\d{2})\([^)]+\) (\d{2}):(\d{2}):(\d{2}).(\d{2})/,
            '<time datetime="$1-$2-$3T$4:$5:$6.$7>$&</time>'
        );

        ret.innerHTML =
            '<header class="meta"><h2>' +
            `<span class="number">${idx}</span> : ` +
            `<span class="name"><b>${fields[NAME]}</b></span> ` +
            `[<span class="mail">${fields[MAIL]}</span>] ` +
            `<span class="date">${datetime}</span>` +
            '</h2></header>';

        const p = document.createElement('p');
        p.className = 'message';
        fields[BODY]
            .split(/(https?:\/\/[a-z\d-.]+?\.[a-z\d-.]+\/[a-z\d_.~!*'();:@&=+$,/?#\[%-\]+]*)/i)
            .forEach((s, i) => {
                if (i % 2) { // URL
                    const a = document.createElement('a');
                    a.href = a.innerText = href;
                    if (location.hostname == a.hostname) {
                        const match = a.pathname.match(PATHNAME_REGEXP);
                        if (match) {
                            const [board, key, range] = match.splice(1);
                            addEventListenerToAnchor(a, href, board, key, range);
                        }
                    }
                    p.appendChild(a);
                } else {
                    p.insertAdjacentHTML(s);
                }
            });

        ret.appendChild(p);

        return ret;
    }

    const CONTAINER = 0;
    const POSTS = 1;
    const BOARD = 2;
    const KEY = 3;
    const RANGE = 4;

    const postsOrig = document.getElementById('posts');

    const cloned = postsOrig.cloneNode();
    postsOrig.parentNode.replaceChild(cloned, postsOrig);
    const posts = [];
    const match = location.pathname.match(PATHNAME_REGEXP);

    let state = [
        cloned, // CONTAINER
        posts, // POSTS
        match[1].toLowerCase(), // BOARD
        match[2], // KEY
        new Ranges(match[3] || ''), // RANGE
    ];
    const states = [state];
    history.replaceState(0, '');
    // A serial number associated with the current state in the history stack.
    let stateId = 0;

    function setClasses() {
        state[RANGE].forEachIndexTo(
            state[POSTS].length,
            (i, inRange) => state[POSTS][i].classList[inRange ? 'add' : 'remove']('post-disabled')
        );
    }

    const threads = Object.create(null);
    threads[`${state[BOARD]}/${state[KEY]}`] = [cloned, posts];

    const reloadButton = document.getElementById('reload-button');
    const submitButton = document.getElementById('submit-button');

    function go(href, board, key, range) {
        board = board.toLowerCase();

        reloadButton.disabled = submitButton.disabled = true;

        const rangeObj = new Ranges(range || '');

        const threadId = `${board}/${key}`;
        const cached = threads[threadId] || (threads[threadId] = [postsOrig.cloneNode(), []]);
        const containerOld = state[CONTAINER];
        state = [...cached, board, key, rangeObj];
        setClasses();
        containerOld.parentNode.replaceChild(state[CONTAINER], containerOld);

        history.pushState(++stateId, '', href);
        states.length = stateId;
        states.push(state);

        const base = `/test/read.cgi/${threadId}/`;
        const forEach = [].forEach;

        forEach.apply(document.getElementsByClassName('board-top'), [a => a.href = `/${board}/`]);

        forEach.apply(document.getElementsByClassName('all'), [a => {
            a.href = base;
            addEventListenerToAnchor(a, board, key);
        }]);

        const min = Math.min(
            rangeObj.fromLast && state[POSTS].length-rangeObj.fromLast,
            rangeObj.intervals[0] || 0
        );
        const rangePrev = `${Math.max(min-100,1)}-${min}`;
        forEach.apply(document.getElementsByClassName('prev'), [a => {
            a.href = base + rangePrev;
            addEventListenerToAnchor(a, board, key, rangePrev);
        }]);

        const max = Math.max(
            rangeObj.fromLast && state[POSTS].length-1,
            rangeObj.intervals ? rangeObj.intervals[rangeObj.intervals.length-1] : 0
        );
        const rangeNext = `${max+2}-${Math.min(max+102,state[POSTS].length)}`;
        forEach.apply(document.getElementsByClassName('next'), [a => {
            a.href = base + rangeNext;
            addEventListenerToAnchor(a, board, key, rangeNext);
        }]);

        const range100 = '-100';
        forEach.apply(document.getElementsByClassName('1-100'), [a => {
            a.href = base + range100;
            addEventListenerToAnchor(a, board, key, range100);
        }]);

        const rangeL50 = 'l50';
        forEach.apply(document.getElementsByClassName('l50'), [a => {
            a.href = base + rangeL50;
            addEventListenerToAnchor(a, board, key, rangeL50);
        }]);

        reload();
    }

    function reload() {
        reloadButton.disabled = submitButton.disabled = true;

        return fetch(`/${state[BOARD]}/dat/${state[KEY]}.dat`).then(
            res => res.text(),
            reason => {
                console.log(`Failed to retrieve the dat file: ${reason}`);
                reloadButton.disabled = submitButton.disabled = false;
            }
        ).then(dat => {
            const lines = dat.split('\n');
            if (! lines[lines.length-1]) {
                lines.pop();
            }

            while (state[POSTS].length < lines.length) {
                const fields = lines[state[POSTS].length].split('<>');
                if (0 == state[POSTS].length) {
                    document.getElementById('title').innerText = document.title = fields[TITLE];
                }
                const elm = createArticle(state[POSTS].length, fields);
                state[CONTAINER].appendChild(elm);
                state[POSTS].push(elm);
            }

            setClasses();

            reloadButton.disabled = submitButton.disabled = false;
        });
    }

    window.onpopstate = () => {
        state = states[history.state];
        setClasses();
        const posts = document.getElementById('posts');
        posts.parentNode.replaceChild(state[CONTAINER], posts);
    };

    const form = document.getElementById('post-form');
    form.addEventListener('submit', e => {
        e.preventDefault();
        const data = new FormData(form);
        data.append('bbs', state[BOARD]);
        data.append('key', state[KEY]);

        fetch('/test/bbs.cgi', { method: 'POST', body: data }).then(
            res => {
                const result = res.headers.get('X-Monaxide-Result');
                if (result && 'SUCCESS' == result.toUpperCase()) {
                    return res.text().then(text => [result, text]);
                }
                form.MESSAGE.value = '';
                return res.text();
            },
            e => console.log(e)
        ).then(
            res => {
                console.log(res);
                return reload();
            },
            e => console.log(e)
        );
    });

    reload();
})();
