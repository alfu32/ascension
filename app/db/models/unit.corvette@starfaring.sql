-- unit.corvette@starfaring: Corvette (mesh.corvette_starfaring)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.corvette_starfaring','Corvette');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.corvette_starfaring';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('hull','box','mat.steel',0,'','{"y":0.2,"z":-0.05,"w":0.64,"d":2,"h":0.34,"taper":0.25,"tint":"#a6b0bd"}'),
  ('prow','box','mat.steel',0,'','{"y":0.22,"z":1.17,"w":0.5,"d":0.5,"h":0.27,"taper":0.75,"tint":"#c3cbd6"}'),
  ('spine','box','mat.steel',1,'','{"y":0.54,"z":-0.25,"w":0.16,"d":1.2,"h":0.05}'),
  ('sponsons','box','mat.steel',0,'','{"x":-0.42,"y":0.2,"z":-0.35,"w":0.22,"d":1.2,"h":0.24,"taper":0.2,"repeat":2,"dx":0.84,"tint":"#8e97a3"}'),
  ('stripes','box','mat.steel',1,'','{"x":-0.42,"y":0.44,"z":-0.35,"w":0.09,"d":1.05,"h":0.025,"repeat":2,"dx":0.84}'),
  ('bridge','box','mat.steel',0,'','{"y":0.54,"z":0.1,"w":0.3,"d":0.34,"h":0.14,"taper":0.3,"tint":"#c3cbd6"}'),
  ('window','box','mat.light_strip',0,'','{"y":0.6,"z":0.27,"w":0.2,"d":0.02,"h":0.04,"far":0}'),
  ('fins','box','mat.steel',1,'','{"x":-0.53,"y":0.22,"z":-0.95,"w":0.04,"d":0.4,"h":0.42,"taper":0.55,"roll":-35,"repeat":1}'),
  ('fin_e','box','mat.steel',1,'','{"x":0.53,"y":0.22,"z":-0.95,"w":0.04,"d":0.4,"h":0.42,"taper":0.55,"roll":35}'),
  ('engine','cylinder','mat.iron',0,'','{"y":0.37,"z":-1.38,"w":0.42,"h":0.36,"segments":8,"pitch":90,"tint":"#4a5260"}'),
  ('drive','wheel','mat.glow',0,'','{"y":0.37,"z":-1.4,"yaw":90,"w":0.34,"t":0.05,"spokes":0,"segments":8}'),
  ('turret','box','mat.steel',0,'','{"x":0,"y":0.54,"z":0.6,"w":0.3,"d":0.32,"h":0.13,"taper":0.25,"traverse":22,"tint":"#9aa4b2"}'),
  ('turret_guns','cylinder','mat.iron',0,'turret','{"x":-0.055,"y":0.06,"z":0.096,"w":0.055,"h":0.42,"segments":4,"pitch":90,"repeat":2,"dx":0.11,"tint":"#4a5260","far":0}'),
  ('turret_flash','box','mat.flash',0,'turret','{"y":0.03,"z":0.536,"w":0.27,"d":0.12,"h":0.1,"flash":1,"far":0}'),
  ('turret_bolts','cylinder','mat.light_strip',0,'turret_flash','{"x":-0.055,"y":0.035,"z":0.14,"w":0.07,"h":0.6,"segments":3,"pitch":90,"repeat":2,"dx":0.11,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.corvette_starfaring'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.corvette'), NULL, (SELECT id FROM def WHERE key='age.starfaring'), (SELECT id FROM def WHERE key='mesh.corvette_starfaring'), (SELECT id FROM def WHERE key='mat.steel'), 100);
