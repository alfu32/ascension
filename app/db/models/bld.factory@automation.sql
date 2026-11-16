-- bld.factory@automation: Future factory (mesh.factory_automation)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.factory_automation','Future factory');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.factory_automation';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.paving',0,'','{"w":3.6,"d":3.6,"h":0.05,"steps":1}'),
  ('hall','box','mat.composite_panel',0,'','{"x":-0.15,"y":0.05,"z":-0.45,"w":3,"d":2.4,"h":1.35}'),
  ('hall_glass','box','mat.smart_glass',0,'','{"x":-0.15,"y":0.7,"z":-0.45,"w":3.02,"d":2.42,"h":0.3}'),
  ('hall_glow','box','mat.light_strip',0,'','{"x":-0.15,"y":0.05,"z":0.76,"w":3,"d":0.03,"h":0.03,"far":0}'),
  ('doors','box','mat.white_composite',0,'','{"x":-1,"y":0.05,"z":0.765,"w":0.55,"d":0.02,"h":0.58,"repeat":3,"dx":0.75,"tint":"#c8d0d4"}'),
  ('roof_slopes','box','mat.solar_panel',0,'','{"x":-0.15,"y":1.4,"z":-1.65,"w":2.92,"d":0.05,"h":0.75,"pitch":53.13,"repeat":4,"dz":0.6}'),
  ('roof_backs','box','mat.white_composite',1,'','{"x":-0.15,"y":1.4,"z":-1.07,"w":2.92,"d":0.04,"h":0.45,"repeat":4,"dz":0.6}'),
  ('parapets','box','mat.white_composite',1,'','{"x":-1.6,"y":1.4,"z":-0.45,"w":0.12,"d":2.4,"h":0.5,"repeat":2,"dx":2.9}'),
  ('parapet_caps','box','mat.light_strip',0,'','{"x":-1.6,"y":1.9,"z":-0.45,"w":0.14,"d":2.42,"h":0.03,"repeat":2,"dx":2.9,"far":0}'),
  ('chimney','cylinder','mat.white_composite',0,'','{"x":1.5,"y":0.05,"z":-1.35,"w":0.5,"h":4.6,"taper":0.35,"segments":10}'),
  ('chimney_band','cylinder','mat.white_composite',1,'','{"x":1.5,"y":3.4,"z":-1.35,"w":0.4,"h":0.48,"segments":10}'),
  ('chimney_glow','cylinder','mat.light_strip',0,'','{"x":1.5,"y":4.6,"z":-1.35,"w":0.34,"h":0.06,"segments":10,"far":0}'),
  ('vent','cylinder','mat.white_composite',0,'','{"x":1.5,"y":0.05,"z":-0.45,"w":0.4,"h":2.4,"segments":10}'),
  ('vent_band','cylinder','mat.white_composite',1,'','{"x":1.5,"y":1.9,"z":-0.45,"w":0.42,"h":0.3,"segments":10}'),
  ('vent_cap','dome','mat.white_composite',0,'','{"x":1.5,"y":2.45,"z":-0.45,"w":0.4,"h":0.16,"segments":10,"far":0}'),
  ('office','dome','mat.smart_glass',0,'','{"x":-1.05,"y":0.05,"z":1.25,"w":1.1,"h":0.75,"segments":10}'),
  ('office_ring','cylinder','mat.white_composite',1,'','{"x":-1.05,"y":0.05,"z":1.25,"w":1.14,"h":0.14,"segments":10}'),
  ('trailer','box','mat.white_composite',1,'','{"x":0.65,"y":0.16,"z":1.25,"w":1.2,"d":0.38,"h":0.42}'),
  ('tractor','box','mat.white_composite',0,'','{"x":1.45,"y":0.12,"z":1.25,"w":0.32,"d":0.38,"h":0.3,"taper":0.2}'),
  ('tractor_glow','box','mat.light_strip',0,'','{"x":1.62,"y":0.24,"z":1.25,"w":0.02,"d":0.3,"h":0.06,"far":0}'),
  ('wheels','cylinder','mat.rubber',0,'','{"x":0.25,"y":0.11,"z":1.05,"w":0.22,"h":0.4,"segments":6,"pitch":90,"repeat":3,"dx":0.55}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.factory_automation'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.factory'), NULL, (SELECT id FROM def WHERE key='age.automation'), (SELECT id FROM def WHERE key='mesh.factory_automation'), (SELECT id FROM def WHERE key='mat.white_composite'), 100);
