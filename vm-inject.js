module.exports = (async () => {
  //##INJECTED_HEADER
  

  // Imports npm packages for this .gbdialog conversational application.

  require('isomorphic-fetch');
  const http = require('node:http');
  const retry = require('async-retry');
  const createRpcClient = require('@push-rpc/core').createRpcClient;
  const createHttpClient = require('@push-rpc/http').createHttpClient;

  // Unmarshalls Local variables from server VM.

  const pid = this.pid;
  let id = this.id;
  let username = this.username;
  let mobile = this.mobile;
  let from = this.from;
  const channel = this.channel;
  const ENTER = this.ENTER;
  const headers = this.headers;
  let httpUsername = this.httpUsername;
  let httpPs = this.httpPs;
  let today = this.today;
  let now = this.now;
  let date = new Date();
  let page = null;
  const files = [];
  let col = 1;
  let index = 1;

  const mid = (arr, start, length) => {
    if (length === undefined) {
      return arr.slice(start);
    }
    return arr.slice(start, start + length);
  };

  // Makes objects in BASIC insensitive.

  const caseInsensitive = listOrRow => {
    if (!listOrRow) {
      return listOrRow;
    }

    const lowercase = oldKey => (typeof oldKey === 'string' ? oldKey.toLowerCase() : oldKey);

    const createCaseInsensitiveProxy = obj => {
      const propertiesMap = new Map(Object.keys(obj).map(propKey => [lowercase(propKey), obj[propKey]]));
      const caseInsensitiveGetHandler = {
        get: (target, property) => propertiesMap.get(lowercase(property))
      };
      return new Proxy(obj, caseInsensitiveGetHandler);
    };

    if (listOrRow.length) {
      return listOrRow.map(row => createCaseInsensitiveProxy(row));
    } else {
      return createCaseInsensitiveProxy(listOrRow);
    }
  };

  // Transfers auto variables into global object.

  for (const key of Object.keys(this.variables)) {
    global[key] = this.variables[key];
    console.log('Defining global variable: ' + key);
  }

  // Defines local utility BASIC functions.

  const ubound = gbarray => {
    let length = 0;
    if (gbarray) {
      length = gbarray.length;
      if (length > 0) {
        if (gbarray[0].gbarray) {
          return length - 1;
        }
      }
    }
    return length;
  };

  const isarray = gbarray => {
    return Array.isArray(gbarray);
  };

  // Proxies remote functions as BASIC functions.

  const weekday = v => {
    return (async () => {
      return await dk.getWeekFromDate({ v });
    })();
  };
  const hour = v => {
    return (async () => {
      return await dk.getHourFromDate({ v });
    })();
  };
  const base64 = v => {
    return (async () => {
      return await dk.getCoded({ v });
    })();
  };
  const tolist = v => {
    return (async () => {
      return await dk.getToLst({ v });
    })();
  };
  const uuid = () => {
    var dt = new Date().getTime();
    var uuid = 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, c => {
      var r = (dt + Math.random() * 16) % 16 | 0;
      dt = Math.floor(dt / 16);
      return (c == 'x' ? r : (r & 0x3) | 0x8).toString(16);
    });
    return uuid;
  };
  const random = () => {
    return Number.parseInt(((Math.random() * 8) % 8) * 100000000);
  };

  // Setups interprocess communication from .gbdialog run-time to the BotServer API.

  const optsRPC = {
    callTimeout: this.callTimeout,
    messageParser: data => {
      return JSON.parse(data);
    }
  };
  let url;
  const agent = http.Agent({ keepAlive: true });
  
  
  url = `http://localhost:${port}/${botId}/dk`;
  const dk = (await createRpcClient(() => createHttpClient(url, { agent: agent }), optsRPC)).remote;
  url = `http://localhost:${port}/${botId}/sys`;
  const sys = (await createRpcClient(() => createHttpClient(url, { agent: agent }), optsRPC)).remote;
  url = `http://localhost:${port}/${botId}/wa`;
  const wa = (await createRpcClient(() => createHttpClient(url, { agent: agent }), optsRPC)).remote;
  url = `http://localhost:${port}/${botId}/img`;
  const img = (await createRpcClient(() => createHttpClient(url, { agent: agent }), optsRPC)).remote;

  const timeout = ms => {
    return new Promise(resolve => setTimeout(resolve, ms));
  };

  const ensureTokens = async firstTime => {
    const REFRESH_THRESHOLD_MS = 10 * 60 * 1000; // 10 minutes in milliseconds
    const tokens = this.tokens ? this.tokens.split(',') : [];

    for (let i = 0; i < tokens.length; i++) {
      const tokenName = tokens[i];

      // Auto update Bearer authentication for the first token.
      const expiresOn = new Date(global[tokenName + '_expiresOn']);
      const expiration = expiresOn.getTime() - REFRESH_THRESHOLD_MS;

      // Expires token 10min. before or if it the first time, load it.
      if (expiration < Date.now() || firstTime) {
        console.log('Expired. Refreshing token...' + expiration);
        try {
          const result = await sys.getCustomToken({ pid: this.pid, tokenName: tokenName });
          global[tokenName] = result.token;
          global[tokenName + '_expiresOn'] = result.expiresOn;
          console.log('DONE:' + new Date(global[tokenName + '_expiresOn']));
        } catch (error) {
          console.error('Failed to refresh token for ' + tokenName + ':', error);
          continue;
        }
      }

      if (i == 0) {
        headers['Authorization'] = 'Bearer ' + global[tokenName];
      }
    }
  };
  const sleep = async ms => {
    return new Promise(resolve => {
      setTimeout(resolve, ms);
    });
  };

  const TOYAML = json => {
    return json;
  };

  // Line of Business logic.

  let __reportMerge = { adds: 0, updates: 0, skipped: 0 };
  let __report = () => {
    return (
      __reportMerge.title +
      ' adds: ' +
      __reportMerge.adds +
      ', updates: ' +
      __reportMerge.updates +
      ' and skipped: ' +
      __reportMerge.skipped +
      '.'
    );
  };
  let REPORT = 'No report yet';

  try {
    await ensureTokens(true);
    
    //##INJECTED_CODE_HERE

  } catch (e) {
    console.log(e);

    reject({ message: e.message, name: e.name });
  } finally {
    // Closes handles if any.

    await wa.closeHandles({ pid: pid });
    await sys.closeHandles({ pid: pid });

    resolve(true);
  }
})();
