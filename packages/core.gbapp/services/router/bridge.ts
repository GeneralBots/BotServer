import fs from 'fs/promises';
import formidable from 'formidable';
import path from 'path';
import bodyParser from 'body-parser';
import express from 'express';
import fetch from 'isomorphic-fetch';
import moment from 'moment';
import * as uuidv4 from 'uuid';
import { IActivity, IBotData, IConversation, IConversationUpdateActivity, IMessageActivity } from './types';
import { GBConfigService } from '../GBConfigService.js';
import { GBUtil } from '../../../../src/util.js';
import urlJoin from 'url-join';
import { GBServer } from '../../../../src/app.js';

const expiresIn = 1800;
const conversationsCleanupInterval = 10000;
const conversations: { [key: string]: IConversation } = {};
const botDataStore: { [key: string]: IBotData } = {};

export const getRouter = (
  serviceUrl: string,
  botUrl: string,
  conversationInitRequired = true,
  botId
): express.Router => {
  const router = express.Router();

  router.use(bodyParser.json()); // for parsing application/json
  router.use(bodyParser.urlencoded({ extended: true })); // for parsing application/x-www-form-urlencoded
  router.use((req, res, next) => {
    res.header('Access-Control-Allow-Origin', '*');
    res.header('Access-Control-Allow-Methods', 'GET, PUT, POST, DELETE, PATCH, OPTIONS');
    res.header(
      'Access-Control-Allow-Headers',
      'Origin, X-Requested-With, Content-Type, Accept, Authorization, x-ms-bot-agent'
    );
    next();
  });

  // CLIENT ENDPOINT
  router.options(`/directline/${botId}/`, (req, res) => {
    res.status(200).end();
  });

  // Creates a conversation
  const reqs = (req, res) => {

    const conversationId: string = uuidv4.v4().toString();
    conversations[conversationId] = {
      conversationId,
      history: []
    };
    console.log('Created conversation with conversationId: ' + conversationId);

    let userId = req.query?.userSystemId ? req.query?.userSystemId : req.body?.user?.id;
    userId = userId ? userId : req.query.userId;

    const activity = createConversationUpdateActivity(serviceUrl, conversationId, userId);

    fetch(botUrl, {
      method: 'POST',
      body: JSON.stringify(activity),
      headers: {
        'Content-Type': 'application/json'
      }
    }).then(response => {
      res.status(response.status).send({
        conversationId,
        expiresIn
      });
    });
  };

  router.post('/v3/directline/conversations/', reqs);
  router.post(`/api/messages/${botId}/v3/directline/conversations/`, reqs);
  router.post(`/directline/${botId}/conversations/`, reqs);
  router.post(`/directline/conversations/`, reqs);

  // Reconnect API
  const req3 = (req, res) => {
    const conversation = getConversation(req.params.conversationId, conversationInitRequired);
    if (conversation) {
      res.status(200).send(conversation);
    } else {
      // Conversation was never initialized
      res.status(400).send();
    }

    console.warn('/v3/directline/conversations/:conversationId not implemented');
  };
  router.get('/v3/directline/conversations/:conversationId', req3);
  router.get(`/directline/${botId}/conversations/:conversationId`, req3);

  // Gets activities from store (local history array for now)
  const req45 = (req, res) => {
    const watermark = req.query.watermark && req.query.watermark !== 'null' ? Number(req.query.watermark) : 0;

    const conversation = getConversation(req.params.conversationId, conversationInitRequired);

    if (conversation) {
      // If the bot has pushed anything into the history array
      if (conversation.history.length > watermark) {
        const activities = conversation.history.slice(watermark);
        res.status(200).json({
          activities,
          watermark: watermark + activities.length
        });
      } else {
        res.status(200).send({
          activities: [],
          watermark
        });
      }
    } else {
      // Conversation was never initialized
      res.status(400).send();
    }
  };

  const req34 = (req, res) => {
    const watermark = req.query.watermark && req.query.watermark !== 'null' ? Number(req.query.watermark) : 0;

    const conversation = getConversation(req.params.conversationId, conversationInitRequired);

    if (conversation) {
      // If the bot has pushed anything into the history array
      if (conversation.history.length > watermark) {
        const activities = conversation.history.slice(watermark);
        res.status(200).json({
          activities,
          watermark: watermark + activities.length
        });
      } else {
        res.status(200).send({
          activities: [],
          watermark
        });
      }
    } else {
      // Conversation was never initialized
      res.status(400).send();
    }
  };

  router.get(`/directline/${botId}/conversations/:conversationId/activities`, req34);
  router.get(`/api/messages/${botId}/v3/directline/conversations/:conversationId/activities`, req34);

  // Sends message to bot. Assumes message activities

  const res2 = (req, res) => {
    const incomingActivity = req.body;
    // Make copy of activity. Add required fields
    const activity = createMessageActivity(incomingActivity, serviceUrl, req.params.conversationId, req.params['pid']);

    const conversation = getConversation(req.params.conversationId, conversationInitRequired);

    if (conversation) {
      conversation.history.push(activity);
      fetch(botUrl, {
        method: 'POST',
        body: JSON.stringify(activity),
        headers: {
          'Content-Type': 'application/json'
        }
      }).then(response => {
        res.status(response.status).json({ id: activity.id });
      });
    } else {
      // Conversation was never initialized
      res.status(400).send();
    }
  };

  // import { createMessageActivity, getConversation } from './yourModule'; // Update this import as needed

  const resupload = async (req, res) => {
    // Extract botId from the URL using the pathname
    const urlParts = req.url.split('/');
    const botId = urlParts[2]; // Assuming the URL is structured like /directline/{botId}/conversations/:conversationId/upload
    const conversationId = req.params.conversationId; // Extract conversationId from parameters

    const uploadDir = path.join(process.cwd(), 'work', `${botId}.gbai`, 'cache'); // Create upload directory path

    // Create the uploads directory if it doesn't exist

    await fs.mkdir(uploadDir, { recursive: true });

    const form = formidable({
      uploadDir, // Use the constructed upload directory
      keepExtensions: true,  // Keep file extensions
    });

    form.parse(req, async (err, fields, files) => {
      if (err) {
        console.log(`Error parsing the file: ${GBUtil.toYAML(err)}.`);
        return res.status(400).send('Error parsing the file.');
      }

      const incomingActivity = fields; // Get incoming activity data
      const file = files.file[0]; // Access the uploaded file

      const fileName = file['newFilename'];
      const fileUrl = urlJoin(GBServer.globals.publicAddress, `${botId}.gbai`,'cache', fileName);

      // Create the activity message
      let userId = req.query?.userSystemId ? req.query?.userSystemId : req.body?.user?.id;
      userId = userId ? userId : req.query?.userId;

      const activity = createMessageActivity(incomingActivity, serviceUrl, conversationId, req.params['pid']);
      activity.from = { id: userId, name: 'webbot' };
      activity.attachments = [{
        contentType: 'application/octet-stream', // Adjust as necessary
        contentUrl: fileUrl,
        name: fileName, // Original filename
      }];
      const conversation = getConversation(conversationId, conversationInitRequired);

      if (conversation) {
        // Add the uploaded file info to the activity
        activity['fileUrl'] = fileUrl; // Set the file URL

        conversation.history.push(activity);

        try {
          const response = await fetch(botUrl, {
            method: 'POST',
            body: JSON.stringify(activity),
            headers: {
              'Content-Type': 'application/json'
            }
          });

          res.status(response.status).json({ id: activity.id });
        } catch (fetchError) {
          console.error('Error fetching bot:', fetchError);
          res.status(500).send('Error processing request.');
        }
      } else {
        // Conversation was never initialized
        res.status(400).send('Conversation not initialized.');
      }
    });
  };

  router.post(`/api/messages/${botId}/v3/directline/conversations/:conversationId/activities`, res2);
  router.post(`/directline/${botId}/conversations/:conversationId/activities`, res2);

  router.post(`/directline/${botId}/conversations/:conversationId/upload`, resupload);

  router.post('/v3/directline/conversations/:conversationId/upload', (req, res) => {
    console.warn('/v3/directline/conversations/:conversationId/upload not implemented');
  });
  router.get('/v3/directline/conversations/:conversationId/stream', (req, res) => {
    console.warn('/v3/directline/conversations/:conversationId/stream not implemented');
  });

  // BOT CONVERSATION ENDPOINT

  router.post('/v3/conversations', (req, res) => {
    console.warn('/v3/conversations not implemented');
  });

  // TODO: Check duplicate. router.post(`/api/messages/${botId}/v3/directline/conversations/:conversationId/activities`, (req, res) => {
  //   let activity: IActivity;

  //   activity = req.body;

  //   const conversation = getConversation(req.params.conversationId, conversationInitRequired);
  //   if (conversation) {
  //     conversation.history.push(activity);
  //     res.status(200).send();
  //   } else {
  //     // Conversation was never initialized
  //     res.status(400).send();
  //   }
  // });

  router.post(`/v3/conversations/:conversationId/activities/:activityId`, (req, res) => {
    let activity: IActivity;

    activity = req.body;
    activity.id = uuidv4.v4();
    activity.from = { id: 'id', name: 'Bot' };

    const conversation = getConversation(req.params.conversationId, conversationInitRequired);
    if (conversation) {
      conversation.history.push(activity);
      res.status(200).send();
    } else {
      // Conversation was never initialized
      res.status(400).send();
    }
  });

  router.get('/v3/conversations/:conversationId/members', (req, res) => {
    console.warn('/v3/conversations/:conversationId/members not implemented');
  });
  router.get('/v3/conversations/:conversationId/activities/:activityId/members', (req, res) => {
    console.warn('/v3/conversations/:conversationId/activities/:activityId/members');
  });

  // BOTSTATE ENDPOINT

  router.get('/v3/botstate/:channelId/users/:userId', (req, res) => {
    console.log('Called GET user data');
    getBotData(req, res);
  });

  router.get('/v3/botstate/:channelId/conversations/:conversationId', (req, res) => {
    console.log('Called GET conversation data');
    getBotData(req, res);
  });

  router.get('/v3/botstate/:channelId/conversations/:conversationId/users/:userId', (req, res) => {
    console.log('Called GET private conversation data');
    getBotData(req, res);
  });

  router.post('/v3/botstate/:channelId/users/:userId', (req, res) => {
    console.log('Called POST setUserData');
    setUserData(req, res);
  });

  router.post('/v3/botstate/:channelId/conversations/:conversationId', (req, res) => {
    console.log('Called POST setConversationData');
    setConversationData(req, res);
  });

  router.post('/v3/botstate/:channelId/conversations/:conversationId/users/:userId', (req, res) => {
    setPrivateConversationData(req, res);
  });

  router.delete('/v3/botstate/:channelId/users/:userId', (req, res) => {
    console.log('Called DELETE deleteStateForUser');
    deleteStateForUser(req, res);
  });

  return router;
};

/**
 * @param app The express app where your offline-directline endpoint will live
 * @param port The port where your offline-directline will be hosted
 * @param botUrl The url of the bot (e.g. http://127.0.0.1:3978/api/messages)
 * @param conversationInitRequired Requires that a conversation is initialized before it is accessed, returning a 400
 * when not the case. If set to false, a new conversation reference is created on the fly. This is true by default.
 */
export const initializeRoutes = (
  app: express.Express,
  port: number,
  botUrl: string,
  conversationInitRequired = true,
  botId
) => {
  conversationsCleanup();

  const directLineEndpoint = `http://127.0.0.1:${port}`;
  const router = getRouter(directLineEndpoint, botUrl, conversationInitRequired, botId);

  app.use(router);
};

const getConversation = (conversationId: string, conversationInitRequired: boolean) => {
  // Create conversation on the fly when needed and init not required
  if (!conversations[conversationId] && !conversationInitRequired) {
    conversations[conversationId] = {
      conversationId,
      history: []
    };
  }
  return conversations[conversationId];
};

const getBotDataKey = (channelId: string, conversationId: string, userId: string) => {
  return `$${channelId || '*'}!${conversationId || '*'}!${userId || '*'}`;
};

const setBotData = (channelId: string, conversationId: string, userId: string, incomingData: IBotData): IBotData => {
  const key = getBotDataKey(channelId, conversationId, userId);
  const newData: IBotData = {
    eTag: new Date().getTime().toString(),
    data: incomingData.data
  };

  if (incomingData) {
    botDataStore[key] = newData;
  } else {
    delete botDataStore[key];
    newData.eTag = '*';
  }

  return newData;
};

const getBotData = (req: express.Request, res: express.Response) => {
  const key = getBotDataKey(req.params.channelId, req.params.conversationId, req.params.userId);
  console.log('Data key: ' + key);

  res.status(200).send(botDataStore[key] || { data: null, eTag: '*' });
};

const setUserData = (req: express.Request, res: express.Response) => {
  res.status(200).send(setBotData(req.params.channelId, req.params.conversationId, req.params.userId, req.body));
};

const setConversationData = (req: express.Request, res: express.Response) => {
  res.status(200).send(setBotData(req.params.channelId, req.params.conversationId, req.params.userId, req.body));
};

const setPrivateConversationData = (req: express.Request, res: express.Response) => {
  res.status(200).send(setBotData(req.params.channelId, req.params.conversationId, req.params.userId, req.body));
};

export const start = (server, botId) => {
  const port = GBConfigService.getServerPort();
  initializeRoutes(server, Number(port), `http://127.0.0.1:${port}/api/messages/${botId}`, null, botId);

  if (botId === 'default') {
    initializeRoutes(server, Number(port), `http://127.0.0.1:${port}/api/messages`, null, botId);
  }
};

const deleteStateForUser = (req: express.Request, res: express.Response) => {
  Object.keys(botDataStore).forEach(key => {
    if (key.endsWith(`!{req.query.userId}`)) {
      delete botDataStore[key];
    }
  });
  res.status(200).send();
};

// CLIENT ENDPOINT HELPERS
const createMessageActivity = (
  incomingActivity: IMessageActivity,
  serviceUrl: string,
  conversationId: string,
  pid
): IMessageActivity => {
  const obj = {
    ...incomingActivity,
    channelId: 'api',
    serviceUrl,
    conversation: { id: conversationId },
    id: uuidv4.v4()
  };
  return obj;
};

const createConversationUpdateActivity = (serviceUrl: string, conversationId: string, userId: any): IConversationUpdateActivity => {
  const activity: IConversationUpdateActivity = {
    type: 'conversationUpdate',
    channelId: 'api',
    serviceUrl,
    conversation: { id: conversationId },
    id: uuidv4.v4(),
    membersAdded: [],
    membersRemoved: [],
    from: { id: userId, name: 'webbot' }
  };

  return activity;
};

const conversationsCleanup = () => {
  setInterval(() => {
    const expiresTime = moment().subtract(expiresIn, 'seconds');
    Object.keys(conversations).forEach(conversationId => {
      if (conversations[conversationId].history.length > 0) {
        const lastTime = moment(
          conversations[conversationId].history[conversations[conversationId].history.length - 1].localTimestamp
        );
        if (lastTime < expiresTime) {
          delete conversations[conversationId];
          console.log('deleted cId: ' + conversationId);
        }
      }
    });
  }, conversationsCleanupInterval);
};
