import axios from 'axios';
import { DataObject, OAuthResponse } from '../../lib/types';

export const init = async ({ body }: DataObject): Promise<OAuthResponse> => {
    try {
        const {
            clientId: client_id,
            clientSecret: client_secret,
            metadata: {
                code,
                formData: { ZENDESK_SUBDOMAIN },
                redirectUri: redirect_uri,
            },
        } = body;

        const requestBody = {
            grant_type: 'authorization_code',
            code,
            client_id,
            client_secret,
            redirect_uri,
            scope: 'read write',
            code_verifier:
                'eNmtK0mRHQ.-93QxgrniB7rb.-TZ_Tjbonygt2aqk1.ltiwXQHmNFeFJPh19MZwXzFDfmFMpUZbAFVtSAtChNU08R4txdDBi7EY6ZHiBp6I8F1drUHZR',
        };

        const response = await axios.post(
            `https://${ZENDESK_SUBDOMAIN}.zendesk.com/oauth/tokens`,
            requestBody,
        );

        const {
            data: { token_type: tokenType, access_token: accessToken },
        } = response;

        const profileResponse = await axios.get(
            `https://${ZENDESK_SUBDOMAIN}.zendesk.com/api/v2/users/me.json`,
            { headers: { Authorization: `Bearer ${accessToken}` } },
        );
        const {
            data: {
                user: { email: ZENDESK_EMAIL_ADDRESS },
            },
        } = profileResponse;

        return {
            accessToken,
            refreshToken: accessToken,
            expiresIn: 2147483647,
            tokenType,
            meta: {
                ZENDESK_EMAIL_ADDRESS,
            },
        };
    } catch (error) {
        throw new Error(`Error fetching access token for Zendesk: ${error}`);
    }
};
