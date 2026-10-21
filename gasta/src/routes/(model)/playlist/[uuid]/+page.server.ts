import { getAllPathsTree } from '#lib/server/sasta_client/index.ts';

export const load = ({ params }) => {
	return {
		uuid: params.uuid,
		filesPromise: getAllPathsTree().then((res) => res.data)
	};
};
