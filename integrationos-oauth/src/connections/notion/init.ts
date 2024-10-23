import axios from 'axios';
import { DataObject, OAuthResponse } from '../../lib/types';
import { differenceInSeconds, generateBasicHeaders } from '../../lib/helpers';

export const init = async ({ body }: DataObject): Promise<OAuthResponse> => {
    try {
        const {
            clientId: client_id,
            clientSecret: client_secret,
            metadata: { code, redirectUri: redirect_uri },
        } = body;

        const requestBody = {
            grant_type: 'authorization_code',
            code,
            redirect_uri,
        };

        const response = await axios.post(
            'https://api.notion.com/v1/oauth/token',
            requestBody,
            {
                headers: {
                    ...generateBasicHeaders(client_id, client_secret),
                    'Content-Type': 'application/json',
                    Accept: 'application/json',
                },
            },
        );

        const {
            access_token: accessToken,
            bot_id: botId,
            workspace_id: workspaceId,
        } = response.data;

        return {
            accessToken,
            refreshToken: accessToken,
            expiresIn: 2147483647,
            tokenType: 'Bearer',
            meta: {
                botId,
                workspaceId,
            },
        };
    } catch (error) {
        throw new Error(`Error fetching access token for Notion: ${error}`);
    }
};
