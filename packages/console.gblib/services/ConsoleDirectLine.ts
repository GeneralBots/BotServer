const Swagger = require('swagger-client');
const rp = require('request-promise');
import { GBService } from 'botlib';

export class ConsoleDirectLine extends GBService {
  public pollInterval = 1000;
  public directLineSecret = '';
  public directLineClientName = 'DirectLineClient';
  public directLineSpecUrl = 'https://docs.botframework.com/en-us/restapi/directline3/swagger.json';

  constructor(directLineSecret) {
    super();

    this.directLineSecret = directLineSecret;

    const directLineClient = rp(this.directLineSpecUrl)
      .then(function(spec) {
        return new Swagger({
          spec: JSON.parse(spec.trim()),
          usePromise: true
        });
      })
      .then(function(client) {
        client.clientAuthorizations.add(
          'AuthorizationBotConnector',
          new Swagger.ApiKeyAuthorization('Authorization', 'Bearer ' + directLineSecret, 'header')
        );

        return client;
      })
      .catch(function(err) {
        console.error('Error initializing DirectLine client', err);
      });

    // TODO: Remove *this* issue.
    const _this_ = this;
    directLineClient.then(client => {
      client.Conversations.Conversations_StartConversation()
        .then(function(response) {
          return response.obj.conversationId;
        })
        .then(function(conversationId) {
          _this_.sendMessagesFromConsole(client, conversationId);
          _this_.pollMessages(client, conversationId);
        })
        .catch(function(err) {
          console.error('Error starting conversation', err);
        });
    });
  }

  public sendMessagesFromConsole(client, conversationId) {
    const _this_ = this;
    process.stdin.resume();
    const stdin = process.stdin;
    process.stdout.write('Command> ');
    stdin.addListener('data', function(e) {
      const input = e.toString().trim();
      if (input) {
        // exit
        if (input.toLowerCase() === 'exit') {
          return process.exit();
        }

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
        }).catch(function(err) {
          console.error('Error sending message:', err);
        });

        process.stdout.write('Command> ');
      }
    });
  }

  /** TBD: Poll Messages from conversation using DirectLine client */
  public pollMessages(client, conversationId) {
    const _this_ = this;
    console.log('Starting polling message for conversationId: ' + conversationId);
    let watermark = null;
    setInterval(function() {
      client.Conversations.Conversations_GetActivities({ conversationId: conversationId, watermark: watermark })
        .then(function(response) {
          watermark = response.obj.watermark;

          return response.obj.activities;
        })
        .then(_this_.printMessages, _this_.directLineClientName);
    }, this.pollInterval);
  }

  public printMessages(activities, directLineClientName) {
    if (activities && activities.length) {
      // ignore own messages
      activities = activities.filter(function(m) {
        return m.from.id !== directLineClientName;
      });

      if (activities.length) {
        // print other messages
        activities.forEach(activity => {
          console.log(activity.text);
        }, this);

        process.stdout.write('Command> ');
      }
    }
  }

  public printMessage(activity) {
    if (activity.text) {
      console.log(activity.text);
    }

    if (activity.attachments) {
      activity.attachments.forEach(function(attachment) {
        switch (attachment.contentType) {
          case 'application/vnd.microsoft.card.hero':
            this.renderHeroCard(attachment);
            break;

          case 'image/png':
            console.log('Opening the requested image ' + attachment.contentUrl);
            open(attachment.contentUrl);
            break;
        }
      });
    }
  }

  public renderHeroCard(attachment) {
    const width = 70;
    const contentLine = function(content) {
      return ' '.repeat((width - content.length) / 2) + content + ' '.repeat((width - content.length) / 2);
    };

    console.log('/' + '*'.repeat(width + 1));
    console.log('*' + contentLine(attachment.content.title) + '*');
    console.log('*' + ' '.repeat(width) + '*');
    console.log('*' + contentLine(attachment.content.text) + '*');
    console.log('*'.repeat(width + 1) + '/');
  }
}
