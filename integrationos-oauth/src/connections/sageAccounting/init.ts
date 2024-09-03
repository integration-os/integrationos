import axios from 'axios';
import { DataObject, OAuthResponse } from '../../lib/types';

export const init = async ({ body }: DataObject): Promise<OAuthResponse> => {
    try {
        const requestBody = {
            grant_type: 'authorization_code',
            code: body.metadata?.code,
            client_id: body.clientId,
            client_secret: body.clientSecret,
            redirect_uri: body.metadata?.redirectUri,
        };

        const response = await axios.post(
            `https://oauth.accounting.sage.com/token`,
            requestBody,
        );

        const {
            data: {
                access_token: accessToken,
                refresh_token: refreshToken,
                expires_in: expiresIn,
                requested_by_id: requestedById,
            },
        } = response;

        return {
            accessToken,
            refreshToken,
            expiresIn,
            tokenType: '',
            meta: {
                requestedById,
            },
        };
    } catch (error) {
        throw new Error(
            `Error fetching access token for SageAccounting: ${error}`,
        );
    }
};
