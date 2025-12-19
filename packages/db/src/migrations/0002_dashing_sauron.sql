CREATE TABLE "tunnels" (
	"subdomain" text PRIMARY KEY NOT NULL,
	"user_id" text NOT NULL,
	"session_id" text NOT NULL,
	"requested_address" text NOT NULL,
	"requested_port" integer NOT NULL,
	"server_port" integer NOT NULL,
	"client_ip" text NOT NULL,
	"created_at" timestamp DEFAULT now() NOT NULL
);
--> statement-breakpoint
ALTER TABLE "tunnels" ADD CONSTRAINT "tunnels_user_id_user_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."user"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
CREATE INDEX "tunnels_user_idx" ON "tunnels" USING btree ("user_id");--> statement-breakpoint
CREATE INDEX "tunnels_session_idx" ON "tunnels" USING btree ("session_id");