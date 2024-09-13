import axios from 'axios';
import qs from 'qs';
import { DataObject, OAuthResponse } from '../../lib/types';
import { generateBasicHeaders } from '../../lib/helpers';

export const refresh = async ({ body }: DataObject): Promise<OAuthResponse> => {
    try {
        const {
            OAUTH_CLIENT_ID: client_id,
            OAUTH_CLIENT_SECRET: client_secret,
            OAUTH_REFRESH_TOKEN: refresh_token,
            OAUTH_REQUEST_PAYLOAD: {
                formData: { NETSUITE_ACCOUNT_ID },
            },
        } = body;

        const requestBody = {
            grant_type: 'refresh_token',
            refresh_token,
        };

        const response = await axios({
            url: `https://${NETSUITE_ACCOUNT_ID}.suitetalk.api.netsuite.com/services/rest/auth/oauth2/v1/token`,
            method: 'POST',
            headers: generateBasicHeaders(client_id, client_secret),
            data: qs.stringify(requestBody),
        });

        const {
            data: {
                access_token: accessToken,
                expires_in: expiresIn,
                token_type: tokenType,
            },
        } = response;

        return {
            accessToken,
            refreshToken: refresh_token,
            expiresIn: +expiresIn,
            tokenType,
            meta: {
                ...body?.OAUTH_METADATA?.meta,
            },
        };
    } catch (error) {
        throw new Error(`Error fetching access token for Netsuite: ${error}`);
    }
};
