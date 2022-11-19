import Swagger from 'swagger-client';
import rp from 'request-promise';
import { GBLog, GBService } from 'botlib';

/**
 * Bot simulator in terminal window.
 */
export class ConsoleDirectLine extends GBService {
  public pollInterval: number = 1000;
  public directLineSecret: string = '';
  public directLineClientName: string = 'DirectLineClient';
  public directLineSpecUrl: string = 'https://docs.botframework.com/en-us/restapi/directline3/swagger.json';

  constructor(directLineSecret: string) {
    super();

    this.directLineSecret = directLineSecret;
    // tslint:disable-next-line:no-unsafe-any
    const directLineClient = rp(this.directLineSpecUrl)
      .then((spec: string) => {
        // tslint:disable-next-line:no-unsafe-any
        return new Swagger({
          spec: JSON.parse(spec.trim()),
          usePromise: true
        });
      })
      .then(client => {
        // tslint:disable-next-line:no-unsafe-any
        client.clientAuthorizations.add(
          'AuthorizationBotConnector',
          // tslint:disable-next-line:no-unsafe-any
          new Swagger.ApiKeyAuthorization('Authorization', `Bearer ${directLineSecret}`, 'header')
        );

        return client;
      })
      .catch(err => {
        GBLog.error(`Error initializing DirectLine client ${err}`);
      });

    const _this_ = this;
    // tslint:disable-next-line:no-unsafe-any
    directLineClient.then(client => {
      // tslint:disable-next-line:no-unsafe-any
      client.Conversations.Conversations_StartConversation()
        .then(response => {
          // tslint:disable-next-line:no-unsafe-any
          return response.obj.conversationId;
        })
        .then(conversationId => {
          _this_.sendMessagesFromConsole(client, conversationId);
          _this_.pollMessages(client, conversationId);
        })
        .catch(err => {
          GBLog.error(`Error starting conversation ${err}`);
        });
    });
  }

  public sendMessagesFromConsole(client, conversationId) {
    const _this_ = this;
    process.stdin.resume();
    const stdin = process.stdin;
    process.stdout.write('Command> ');
    stdin.addListener('data', e => {
      // tslint:disable-next-line:no-unsafe-any
      const input: string = e.toString().trim();
      if (input !== undefined) {
        // exit
        if (input.toLowerCase() === 'exit') {
          return process.exit();
        }

        // tslint:disable-next-line:no-unsafe-any
        client.Conversations.Conversations_PostActivity({
          conversationId: conversationId,
          activity: {
            textFormat: 'plain',
            text: input,
            type: 'message',
            from: {
              id: _this_.directLineClientName,
              name: _this_.directLineClientName
            }
          }
        }).catch(err => {
          GBLog.error(`Error sending message: ${err}`);
        });

        process.stdout.write('Command> ');
      }
    });
  }

  public pollMessages(client, conversationId) {
    const _this_ = this;
    GBLog.info(`Starting polling message for conversationId: ${conversationId}`);
    let watermark;
    setInterval(() => {
      // tslint:disable-next-line:no-unsafe-any
      client.Conversations.Conversations_GetActivities({ conversationId: conversationId, watermark: watermark })
        .then(response => {
          // tslint:disable-next-line:no-unsafe-any
          watermark = response.obj.watermark;

          // tslint:disable-next-line:no-unsafe-any
          return response.obj.activities;
        })
        .then(_this_.printMessages, _this_.directLineClientName);
      // tslint:disable-next-line:align
    }, this.pollInterval);
  }

  // tslint:disable:no-unsafe-any
  public printMessages(activities, directLineClientName) {
    if (activities && activities.length) {
      // ignore own messages
      activities = activities.filter(m => {
        return m.from.id !== directLineClientName;
      });

      if (activities.length) {
        // print other messages
        activities.forEach(activity => {
          GBLog.info(activity.text);
          // tslint:disable-next-line:align
        }, this);

        process.stdout.write('Command> ');
      }
    }
  }
  // tslint:enable:no-unsafe-any

  // tslint:disable:no-unsafe-any
  public printMessage(activity) {
    if (activity.text) {
      GBLog.info(activity.text);
    }

    if (activity.attachments) {
      activity.attachments.forEach(attachment => {
        switch (attachment.contentType) {
          case 'application/vnd.microsoft.card.hero':
            this.renderHeroCard(attachment);
            break;

          case 'image/png':
            GBLog.info(`Opening the requested image ${attachment.contentUrl}`);
            open(attachment.contentUrl);
            break;

          default:
            GBLog.info(`Unknown contentType: ${attachment.contentType}`);
            break;
        }
      });
    }
  }
  // tslint:enable:no-unsafe-any

  // tslint:disable:no-unsafe-any
  public renderHeroCard(attachment) {
    const width = 70;
    const contentLine = content => {
      return `${' '.repeat((width - content.length) / 2)}content${' '.repeat((width - content.length) / 2)}`;
    };

    GBLog.info(`/${'*'.repeat(width + 1)}`);
    GBLog.info(`*${contentLine(attachment.content.title)}*`);
    GBLog.info(`*${' '.repeat(width)}*`);
    GBLog.info(`*${contentLine(attachment.content.text)}*`);
    GBLog.info(`${'*'.repeat(width + 1)}/`);
  }
  // tslint:enable:no-unsafe-any
}
