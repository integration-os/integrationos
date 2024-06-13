import axios from "axios";
import qs from "qs";
import { DataObject, OAuthResponse } from "../../lib/types";

export const init = async ({ body }: DataObject): Promise<OAuthResponse> => {
  try {
    const requestBody = {
      code: body.metadata?.code,
      client_id: body.clientId,
      client_secret: body.clientSecret,
      redirect_uri: body.metadata?.redirectUri,
    };

    const response = await axios({
      url: `https://slack.com/api/oauth.v2.access`,
      method: "POST",
      headers: { "Content-Type": "application/x-www-form-urlencoded" },
      data: qs.stringify(requestBody),
    });

    const {
      data: {
        access_token,
        app_id: appId,
        bot_user_id: botUserId,
        team: { id: teamId, name: teamName },
        incoming_webhook: { channel, channel_id: channelId },
      },
    } = response;

    return {
      accessToken: access_token,
      refreshToken: access_token,
      expiresIn: 2147483647,
      tokenType: "Bearer",
      meta: {
        appId,
        botUserId,
        teamId,
        teamName,
        channel,
        channelId,
      },
    };
  } catch (error) {
    throw new Error(`Error fetching access token for Slack: ${error}`);
  }
};
