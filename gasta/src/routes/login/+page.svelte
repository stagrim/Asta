<script lang="ts">
	import { Button } from '$lib/components/ui/button/index.js';
	import * as Card from '$lib/components/ui/card/index.js';
	import { FieldGroup, Field, FieldDescription } from '$lib/components/ui/field/index.js';
	import { FileSliders } from '@lucide/svelte';
	import { signIn } from '@auth/sveltekit/client';

	let { data } = $props();
</script>

<div class="flex h-full flex-col items-center md:p-10">
	<div class="flex w-full max-w-sm flex-col gap-6">
		<div class="flex items-center gap-2 self-center font-medium">
			<FileSliders class="size-4" />
			Asta Admin Panel
		</div>
		<div class="flex flex-col gap-6">
			<Card.Root>
				<Card.Header class="text-center">
					<Card.Title class="text-xl">Welcome back</Card.Title>
					<Card.Description>Login with your Dsek account</Card.Description>
				</Card.Header>
				<Card.Content>
					<form>
						<FieldGroup>
							<Field>
								<Button variant="outline" type="button" onclick={() => signIn('authentik')}>
									<svg
										width="32"
										height="44"
										viewBox="0 0 32 44"
										fill="none"
										xmlns="http://www.w3.org/2000/svg"
										><path
											d="M23.3708 7.4382V5.82022H26.7865C27.1461 6.17977 27.5056 6.7191 27.8652 7.25843L23.3708 7.4382ZM29.4831 6.17977C27.5056 2.58427 23.5506 0.786516 17.7978 0.786516C16.1798 0.786516 14.9213 0.966292 13.6629 1.14607V0.426966H12.9438L0.359551 6.17977V9.23596C1.25843 9.23596 1.97753 9.59551 2.51685 10.1348C3.05618 10.6742 3.41573 11.3933 3.41573 12.2921V37.2809V36.9213C3.41573 37.8202 3.05618 38.5393 2.51685 39.0787C1.97753 39.618 1.25843 39.9775 0.359551 39.9775H0V43.573H17.7978C23.5506 43.573 27.5056 41.7753 29.4831 38.1798C31.1011 35.3034 32 29.9101 32 22.1798C32 14.4494 31.1011 9.05618 29.4831 6.17977ZM23.5506 11.0337V9.5955H28.9438C29.1236 10.3146 29.3034 10.8539 29.3034 11.0337H23.5506ZM23.5506 14.4494V13.0112H29.8427C29.8427 13.3708 30.0225 13.9101 30.0225 14.6292L23.5506 14.4494ZM23.5506 18.0449V16.6067H30.2022V18.2247H23.5506V18.0449ZM23.5506 21.6404V20.0225H30.382V21.6404H23.5506ZM30.2022 25.236H23.5506V23.618H30.2022V25.236ZM6.11236 6.17977H4.49438L7.01124 4.92135C6.65169 5.2809 6.47191 5.82022 6.11236 6.17977ZM30.2022 28.8315H23.5506V27.2135H30.2022V28.8315ZM21.2135 41.236C20.8539 41.5955 20.3146 41.9551 19.7753 41.9551H18.3371V40.1573H16.7191C15.8202 40.1573 15.1011 39.7978 14.5618 39.2584C14.0225 38.7191 13.6629 38 13.6629 37.1011V4.20225C13.8427 3.12359 14.5618 2.58427 15.6404 2.58427H19.7753C20.3146 2.58427 20.8539 2.76404 21.2135 3.12359C21.573 3.48315 21.7528 4.02247 21.7528 4.5618V39.7978C21.9326 40.3371 21.7528 40.8764 21.2135 41.236ZM4.67416 9.5955C4.31461 9.05618 3.95506 8.69663 3.41573 8.1573H5.21348C4.85393 8.8764 4.85393 9.23595 4.67416 9.5955ZM29.6629 32.427H23.5506V30.809H30.0225C29.8427 31.5281 29.6629 32.0674 29.6629 32.427ZM28.764 36.0225H23.5506V34.4045H29.3034L28.764 36.0225ZM26.2472 39.4382H23.3708C23.3708 39.0787 23.3708 38.5393 23.5506 37.8202H27.8652L26.2472 39.4382ZM5.21348 38.7191C5.39326 38.1798 5.57303 37.8202 5.57303 37.2809C5.93258 38 6.11236 38.5393 6.29214 38.7191H5.21348ZM1.97753 41.9551V41.7753C2.33708 41.5955 2.8764 41.4157 3.59551 40.8764H8.08989C8.62921 41.236 9.16854 41.7753 9.52809 41.9551H1.97753Z"
											fill="white"
										></path></svg
									>
									Login with Dsek
								</Button>
								{#if data.isInDevEnvironment}
									<Button variant="outline" type="button" onclick={() => signIn('credentials')}>
										Login with hopes and prayers
									</Button>
								{/if}
							</Field>
						</FieldGroup>
					</form>
				</Card.Content>
			</Card.Root>
			<FieldDescription class="px-6 text-center">
				By clicking continue, you agree that you will behave yourself :)
			</FieldDescription>
		</div>
	</div>
</div>
