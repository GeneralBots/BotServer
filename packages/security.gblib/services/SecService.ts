const Fs = require('fs');
import urlJoin = require('url-join');

import { GBService, IGBInstance } from 'botlib';
import { GuaribasGroup, GuaribasUser, GuaribasUserGroup } from '../models';
import { ConversationReference } from 'botbuilder';

/**
 * Security service layer.
 */
export class SecService extends GBService {
  public async importSecurityFile(localPath: string, instance: IGBInstance) {
    const security = JSON.parse(Fs.readFileSync(urlJoin(localPath, 'security.json'), 'utf8'));
    security.groups.forEach(group => {
      const groupDb = GuaribasGroup.build({
        instanceId: instance.instanceId,
        displayName: group.displayName
      });
      groupDb.save().then(g1 => {
        group.users.forEach(user => {
          const userDb = GuaribasUser.build({
            instanceId: instance.instanceId,
            groupId: g1.groupId,
            userName: user.userName
          });
          userDb.save().then(user2 => {
            const userGroup = GuaribasUserGroup.build();
            userGroup.groupId = g1.groupId;
            userGroup.userId = user2.userId;
            userGroup.save();
          });
        });
      });
    });
  }

  public async ensureUser(
    instanceId: number,
    userSystemId: string,
    currentBotId: string,
    userName: string,
    address: string,
    channelName: string,
    displayName: string,
    phone: string
  ): Promise<GuaribasUser> {
    let user = await GuaribasUser.findOne({
      
      where: {
        instanceId: instanceId,
        userSystemId: userSystemId
      }
    });

    if (!user) {
      user = GuaribasUser.build();
    }

    user.instanceId = instanceId;
    user.userSystemId = userSystemId;
    user.currentBotId = currentBotId;
    user.userName = userName;
    user.displayName = displayName;
    user.internalAddress = address;
    user.email = userName;
    user.phone = phone;
    user.defaultChannel = channelName;
    user.save();
    return user;
  }

  /**
   * Retrives a conversation reference from contact phone.
   */
  public async getConversationReference(phone: string): Promise<ConversationReference> {
    const options = { where: { phone: phone } };
    const user = await GuaribasUser.findOne(options);

    return JSON.parse(user.conversationReference);
  }

  /**
   * Updates a conversation reference from contact phone.
   */
  public async updateConversationReference(phone: string, conversationReference: string) {
    const options = { where: { phone: phone } };
    const user = await GuaribasUser.findOne(options);

    user.conversationReference = conversationReference;
    await user.save();
  }

  public async updateCurrentBotId(
    instanceId: number,
    userSystemId: string,
    currentBotId: string
  ): Promise<GuaribasUser> {
    let user = await GuaribasUser.findOne({
      where: { 
        instanceId: instanceId,
        userSystemId: userSystemId
      }
    });
    user.currentBotId = currentBotId;
    await user.save();
    return user;
  }

  public async getUserFromPhone(
    phone: string
  ): Promise<GuaribasUser> {
    return await GuaribasUser.findOne({
      where: {
        phone: phone
      }
    });
  }


}
